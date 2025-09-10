use std::{error::Error, fmt::Display};

use api_derive::APIEndpointError;
use axum::{
    Json,
    extract::{FromRequest, State},
    http::StatusCode,
    response::{Sse, sse::Event},
};
use serde::Serialize;

use api_core::{APIErrorCode, APIHandler, APIResult};
use tokio_stream::{StreamExt, adapters::FilterMap, wrappers::WatchStream};

use crate::{AppState, tokens::AuthUser, vote_auth::VoteState};

#[derive(FromRequest)]
pub struct VoteActiveRequest {
    auth: AuthUser,
    state: State<AppState>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VoteActiveResponse {
    is_active: bool,
}

#[derive(APIEndpointError)]
#[api(endpoint(method = "GET", path = "/api/common/vote-state-watch"))]
pub enum VoteActiveError {
    #[api(code = APIErrorCode::MUIDNotFound, status = 404)]
    MUIDNotFound,
}

pub struct VoteActive;
impl APIHandler for VoteActive {
    type State = AppState;
    type Request = VoteActiveRequest;

    const SUCCESS_CODE: StatusCode = StatusCode::OK;
    type SuccessResponse = Json<VoteActiveResponse>;
    type ErrorResponse = VoteActiveError;
    async fn route(
        request: Self::Request,
    ) -> APIResult<Self::SuccessResponse, Self::ErrorResponse> {
        let VoteActiveRequest {
            auth:
                AuthUser {
                    uuid,
                    muid,
                    is_host,
                },
            state: State(state),
        } = request;

        let res = if let Some(meeting) = state.meetings.lock().await.get(&muid) {
            meeting.vote_auth.is_active()
        } else {
            return Err(VoteActiveError::MUIDNotFound);
        };

        Ok(Json(VoteActiveResponse { is_active: res }))
    }
}

#[derive(FromRequest)]
pub struct VoteStateWatchRequest {
    auth: AuthUser,
    state: State<AppState>,
}

#[derive(APIEndpointError, Debug)]
#[api(endpoint(method = "GET", path = "/api/common/vote-watch"))]
pub enum VoteStateWatchError {
    #[api(code = APIErrorCode::MUIDNotFound, status = 404)]
    MUIDNotFound,
}
impl Display for VoteStateWatchError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}
impl Error for VoteStateWatchError {}

/// Endpoint for waiting for voting round to begin.
///
/// Returns 200 OK along with a server-side-event upon success
pub struct VoteStateWatch;
impl APIHandler for VoteStateWatch {
    type State = AppState;
    type Request = VoteStateWatchRequest;

    const SUCCESS_CODE: StatusCode = StatusCode::OK;
    type SuccessResponse = Sse<
        FilterMap<
            WatchStream<VoteState>,
            fn(VoteState) -> Option<Result<Event, VoteStateWatchError>>,
        >,
    >;
    type ErrorResponse = VoteStateWatchError;

    async fn route(
        request: Self::Request,
    ) -> APIResult<Self::SuccessResponse, Self::ErrorResponse> {
        let VoteStateWatchRequest {
            auth:
                AuthUser {
                    uuid,
                    muid,
                    is_host,
                },
            state: State(state),
        } = request;

        let upon_event = |new_state: VoteState| match new_state {
            VoteState::Creation => Some(Ok::<Event, VoteStateWatchError>(
                Event::default().data("Creation"),
            )),
            VoteState::Voting => Some(Ok::<Event, VoteStateWatchError>(
                Event::default().data("Voting"),
            )),
            VoteState::Tally => Some(Ok::<Event, VoteStateWatchError>(
                Event::default().data("Tally"),
            )),
        };

        if let Some(meeting) = state.meetings.lock().await.get(&muid) {
            let state_rx = meeting.vote_auth.new_watcher();
            let stream = WatchStream::new(state_rx).filter_map(upon_event as _);
            Ok(Sse::new(stream))
        } else {
            Err(VoteStateWatchError::MUIDNotFound)
        }
    }
}
