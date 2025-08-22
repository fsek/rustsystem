use std::{error::Error, fmt::Display, io};

use axum::{
    Json,
    extract::{FromRequest, State},
    http::StatusCode,
    response::{IntoResponse, Response, Sse, sse::Event},
};
use serde::{Deserialize, Serialize};
use tokio_stream::{StreamExt, adapters::FilterMap, wrappers::WatchStream};

use crate::{AppState, api::APIHandler};

use super::auth::AuthHost;

#[derive(FromRequest)]
pub struct InviteWatchRequest {
    auth: AuthHost,
    state: State<AppState>,
}

#[derive(Serialize, Debug)]
pub enum InviteWatchError {
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
    type SuccessResponse =
        Sse<FilterMap<WatchStream<bool>, fn(bool) -> Option<Result<Event, InviteWatchError>>>>;
    type ErrorResponse = Json<InviteWatchError>;
    async fn handler(
        request: Self::Request,
    ) -> crate::api::APIResponse<Self::SuccessResponse, Self::ErrorResponse> {
        let InviteWatchRequest {
            auth: AuthHost { uuid, muid },
            state: State(state),
        } = request;
        if let Some(meeting) = state.meetings.lock().await.get(&muid) {
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

            return Ok((StatusCode::OK, Sse::new(stream)));
        } else {
            return Err((StatusCode::NOT_FOUND, Json(InviteWatchError::MUIDNotFound)));
        }
    }
}
