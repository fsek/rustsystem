use async_trait::async_trait;
use axum::{
    extract::{FromRequest, State},
    http::StatusCode,
    response::{Sse, sse::Event},
};
use tokio_stream::{StreamExt, adapters::FilterMap, wrappers::WatchStream};

use rustsystem_core::{APIError, APIHandler, Method};

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

        let upon_event = |new_state| {
            if new_state {
                Some(Ok::<Event, APIError>(Event::default().data("Ready")))
            } else {
                Some(Ok::<Event, APIError>(Event::default().data("Wait")))
            }
        };

        let meeting = state.get_meeting(auth.muuid).await?;
        let state_rx = meeting.invite_auth.read().await.new_watcher();
        let stream = WatchStream::new(state_rx).filter_map(upon_event as _);
        Ok(Sse::new(stream))
    }
}
