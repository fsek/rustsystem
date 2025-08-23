use std::{error::Error, fmt::Display};

use axum::{
    Json,
    extract::{FromRequest, State},
    http::StatusCode,
    response::{Sse, sse::Event},
};
use serde::Serialize;
use tokio_stream::{Stream, StreamExt, adapters::FilterMap, wrappers::WatchStream};

use api_core::{APIHandler, APIResponse};

use crate::AppState;

use super::auth::AuthVoter;

#[derive(FromRequest)]
pub struct VoteWatchRequest {
    auth: AuthVoter,
    state: State<AppState>,
}

#[derive(Serialize, Debug)]
pub enum VoteWatchError {
    MUIDNotFound,
}
impl Display for VoteWatchError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}
impl Error for VoteWatchError {}

/// Endpoint for waiting for voting round to begin.
///
/// Returns 200 OK along with a server-side-event upon success
pub struct VoteWatch;
impl APIHandler for VoteWatch {
    type State = AppState;
    type Request = VoteWatchRequest;
    type SuccessResponse =
        Sse<FilterMap<WatchStream<bool>, fn(bool) -> Option<Result<Event, VoteWatchError>>>>;
    type ErrorResponse = Json<VoteWatchError>;

    async fn handler(
        request: Self::Request,
    ) -> APIResponse<Self::SuccessResponse, Self::ErrorResponse> {
        let VoteWatchRequest {
            auth: AuthVoter { uuid, muid },
            state: State(state),
        } = request;

        let upon_event = |new_state| {
            if new_state {
                Some(Ok::<Event, VoteWatchError>(Event::default().data("Start")))
            } else {
                Some(Ok::<Event, VoteWatchError>(Event::default().data("Stop")))
            }
        };

        if let Some(meeting) = state.meetings.lock().await.get(&muid) {
            let state_rx = meeting.vote_auth.new_watcher();
            let stream = WatchStream::new(state_rx).filter_map(upon_event as _);
            return Ok((StatusCode::OK, Sse::new(stream)));
        } else {
            return Err((StatusCode::NOT_FOUND, Json(VoteWatchError::MUIDNotFound)));
        }
    }
}
