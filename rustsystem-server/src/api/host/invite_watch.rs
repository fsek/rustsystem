use async_trait::async_trait;
use axum::{
    extract::{FromRequest, State},
    http::StatusCode,
    response::{Sse, sse::Event},
};
use tokio_stream::{StreamExt, adapters::FilterMap, wrappers::WatchStream};

use api_core::{APIError, APIErrorCode, APIHandler, Method};

use crate::AppState;

use super::auth::AuthHost;

#[derive(FromRequest)]
pub struct InviteWatchRequest {
    auth: AuthHost,
    state: State<AppState>,
}

pub struct InviteWatch;
#[async_trait]
impl APIHandler for InviteWatch {
    type State = AppState;
    type Request = InviteWatchRequest;

    const METHOD: Method = Method::Get;
    const PATH: &'static str = "/invite-watch";
    const SUCCESS_CODE: StatusCode = StatusCode::OK;
    type SuccessResponse =
        Sse<FilterMap<WatchStream<bool>, fn(bool) -> Option<Result<Event, APIError>>>>;
    async fn route(request: Self::Request) -> Result<Self::SuccessResponse, APIError> {
        let InviteWatchRequest {
            auth,
            state: State(state),
        } = request;

        let meetings_guard = {
            let guard = state.read()?;
            guard.clone().meetings
        };

        if let Some(meeting) = meetings_guard.lock().await.get(&auth.muuid) {
            let state_rx = meeting.invite_auth.new_watcher();

            let upon_event = |new_state| {
                if new_state {
                    Some(Ok::<Event, APIError>(Event::default().data("Ready")))
                } else {
                    Some(Ok::<Event, APIError>(Event::default().data("Wait")))
                }
            };

            let stream = WatchStream::new(state_rx).filter_map(upon_event as _);

            Ok(Sse::new(stream))
        } else {
            Err(APIError::from_error_code(APIErrorCode::MUuidNotFound))
        }
    }
}
