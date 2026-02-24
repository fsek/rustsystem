use async_trait::async_trait;
use axum::{
    extract::{FromRequest, State},
    http::StatusCode,
    response::{Sse, sse::Event},
};

use rustsystem_core::{APIError, APIErrorCode, APIHandler, Method};
use tokio_stream::{StreamExt, adapters::FilterMap, wrappers::WatchStream};

use crate::{AppState, tokens::AuthUser};

#[derive(FromRequest)]
pub struct MeetingSpecsWatchRequest {
    auth: AuthUser,
    state: State<AppState>,
}

pub struct MeetingSpecsWatch;
#[async_trait]
impl APIHandler for MeetingSpecsWatch {
    type State = AppState;
    type Request = MeetingSpecsWatchRequest;

    const METHOD: Method = Method::Get;
    const PATH: &'static str = "/meeting-specs-watch";
    const SUCCESS_CODE: StatusCode = StatusCode::OK;
    type SuccessResponse =
        Sse<FilterMap<WatchStream<bool>, fn(bool) -> Option<Result<Event, APIError>>>>;

    async fn route(request: Self::Request) -> Result<Self::SuccessResponse, APIError> {
        let MeetingSpecsWatchRequest {
            auth,
            state: State(state),
        } = request;

        let upon_event = |new_state: bool| {
            if new_state {
                Some(Ok::<Event, APIError>(Event::default().data("NewData")))
            } else {
                // This should never be called. The frontend will not recognize it!
                Some(Ok::<Event, APIError>(Event::default().data("DataFailure")))
            }
        };

        let meetings = state.meetings()?;

        if let Some(meeting) = meetings.lock().await.get(&auth.muuid) {
            let update_rx = meeting.vote_auth.new_update_watcher();
            let stream = WatchStream::new(update_rx).filter_map(upon_event as _);
            Ok(Sse::new(stream))
        } else {
            Err(APIError::from_error_code(APIErrorCode::MUuidNotFound))
        }
    }
}
