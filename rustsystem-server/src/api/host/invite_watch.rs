use std::{error::Error, fmt::Display};

use api_derive::APIEndpointError;
use axum::{
    extract::{FromRequest, State},
    http::StatusCode,
    response::{Sse, sse::Event},
};
use tokio_stream::{StreamExt, adapters::FilterMap, wrappers::WatchStream};

use api_core::{APIErrorCode, APIHandler, APIResult};

use crate::AppState;

use super::auth::AuthHost;

#[derive(FromRequest)]
pub struct InviteWatchRequest {
    auth: AuthHost,
    state: State<AppState>,
}

#[derive(APIEndpointError, Debug)]
#[api(endpoint(method = "GET", path = "/api/host/invite-watch"))]
pub enum InviteWatchError {
    #[api(code = APIErrorCode::MUuidNotFound, status = 404)]
    MUIDNotFound,
}
impl Display for InviteWatchError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}
impl Error for InviteWatchError {}

pub struct InviteWatch;
impl APIHandler for InviteWatch {
    type State = AppState;
    type Request = InviteWatchRequest;

    const SUCCESS_CODE: StatusCode = StatusCode::OK;
    type SuccessResponse =
        Sse<FilterMap<WatchStream<bool>, fn(bool) -> Option<Result<Event, InviteWatchError>>>>;
    type ErrorResponse = InviteWatchError;
    async fn route(
        request: Self::Request,
    ) -> APIResult<Self::SuccessResponse, Self::ErrorResponse> {
        let InviteWatchRequest {
            auth: AuthHost { uuuid, muuid },
            state: State(state),
        } = request;
        if let Some(meeting) = state.meetings.lock().await.get(&muuid) {
            let state_rx = meeting.invite_auth.new_watcher();

            let upon_event = |new_state| {
                if new_state {
                    Some(Ok::<Event, InviteWatchError>(
                        Event::default().data("Ready"),
                    ))
                } else {
                    Some(Ok::<Event, InviteWatchError>(Event::default().data("Wait")))
                }
            };

            let stream = WatchStream::new(state_rx).filter_map(upon_event as _);

            Ok(Sse::new(stream))
        } else {
            Err(InviteWatchError::MUIDNotFound)
        }
    }
}
