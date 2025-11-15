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
#[api(endpoint(method = "GET", path = "/api/common/vote-active"))]
pub enum VoteActiveError {
    #[api(code = APIErrorCode::MUuidNotFound, status = 404)]
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
            auth,
            state: State(state),
        } = request;

        let res = if let Some(meeting) = state.meetings.lock().await.get(&auth.muuid) {
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
#[api(endpoint(method = "GET", path = "/api/common/vote-state-watch"))]
pub enum VoteStateWatchError {
    #[api(code = APIErrorCode::MUuidNotFound, status = 404)]
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
            auth,
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

        if let Some(meeting) = state.meetings.lock().await.get(&auth.muuid) {
            let state_rx = meeting.vote_auth.new_state_watcher();
            let stream = WatchStream::new(state_rx).filter_map(upon_event as _);
            Ok(Sse::new(stream))
        } else {
            Err(VoteStateWatchError::MUIDNotFound)
        }
    }
}

#[derive(FromRequest)]
pub struct VoteProgressRequest {
    auth: AuthUser,
    state: State<AppState>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VoteProgressResponse {
    is_active: bool,
    is_tally: bool,
    total_votes_cast: usize,
    total_participants: usize,
    vote_name: Option<String>,
}

#[derive(APIEndpointError)]
#[api(endpoint(method = "GET", path = "/api/common/vote-progress"))]
pub enum VoteProgressError {
    #[api(code = APIErrorCode::MUuidNotFound, status = 404)]
    MUIDNotFound,
}

pub struct VoteProgress;
impl APIHandler for VoteProgress {
    type State = AppState;
    type Request = VoteProgressRequest;

    const SUCCESS_CODE: StatusCode = StatusCode::OK;
    type SuccessResponse = Json<VoteProgressResponse>;
    type ErrorResponse = VoteProgressError;
    async fn route(
        request: Self::Request,
    ) -> APIResult<Self::SuccessResponse, Self::ErrorResponse> {
        let VoteProgressRequest {
            auth,
            state: State(state),
        } = request;

        if let Some(meeting) = state.meetings.lock().await.get(&auth.muuid) {
            let is_active = meeting.vote_auth.is_active();
            let is_tally = meeting.vote_auth.is_tally();

            let total_participants = meeting.voters.len();
            let (total_votes_cast, vote_name) = if is_active || is_tally {
                if let Some(round) = meeting.vote_auth.round_ref() {
                    let votes_cast = round.get_vote_count();
                    let name = meeting.vote_auth.get_current_vote_name().map(|s| s.clone());
                    (votes_cast, name)
                } else {
                    (0, None)
                }
            } else {
                (0, None)
            };

            Ok(Json(VoteProgressResponse {
                is_active,
                is_tally,
                total_votes_cast,
                total_participants,
                vote_name,
            }))
        } else {
            Err(VoteProgressError::MUIDNotFound)
        }
    }
}

#[derive(FromRequest)]
pub struct VoteProgressWatchRequest {
    auth: AuthUser,
    state: State<AppState>,
}

#[derive(APIEndpointError, Debug)]
#[api(endpoint(method = "GET", path = "/api/common/vote-progress-watch"))]
pub enum VoteProgressWatchError {
    #[api(code = APIErrorCode::MUuidNotFound, status = 404)]
    MUIDNotFound,
}

impl Display for VoteProgressWatchError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}

impl Error for VoteProgressWatchError {}

/// Endpoint for watching vote progress updates in real-time
pub struct VoteProgressWatch;
impl APIHandler for VoteProgressWatch {
    type State = AppState;
    type Request = VoteProgressWatchRequest;

    const SUCCESS_CODE: StatusCode = StatusCode::OK;
    type SuccessResponse = Sse<
        FilterMap<WatchStream<bool>, fn(bool) -> Option<Result<Event, VoteProgressWatchError>>>,
    >;
    type ErrorResponse = VoteProgressWatchError;

    async fn route(
        request: Self::Request,
    ) -> APIResult<Self::SuccessResponse, Self::ErrorResponse> {
        let VoteProgressWatchRequest {
            auth,
            state: State(state),
        } = request;

        let upon_event = |_update: bool| {
            Some(Ok::<Event, VoteProgressWatchError>(
                Event::default().data("VoteProgressUpdated"),
            ))
        };

        if let Some(meeting) = state.meetings.lock().await.get(&auth.muuid) {
            let update_rx = meeting.vote_auth.new_update_watcher();
            let stream = WatchStream::new(update_rx).filter_map(upon_event as _);
            Ok(Sse::new(stream))
        } else {
            Err(VoteProgressWatchError::MUIDNotFound)
        }
    }
}
