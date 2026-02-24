use async_trait::async_trait;
use axum::{
    extract::{FromRequest, State},
    http::StatusCode,
    response::{Sse, sse::Event},
};

use rustsystem_core::{APIError, APIErrorCode, APIHandler, Method};
use tokio_stream::{StreamExt, adapters::FilterMap, wrappers::WatchStream};

use crate::{AppState, tokens::AuthUser, vote_auth::VoteState};

#[derive(FromRequest)]
pub struct VoteStateWatchRequest {
    auth: AuthUser,
    state: State<AppState>,
}

/// Endpoint for waiting for voting round to begin.
///
/// Returns 200 OK along with a server-side-event upon success
pub struct VoteStateWatch;
#[async_trait]
impl APIHandler for VoteStateWatch {
    type State = AppState;
    type Request = VoteStateWatchRequest;

    const METHOD: Method = Method::Get;
    const PATH: &'static str = "/vote-state-watch";
    const SUCCESS_CODE: StatusCode = StatusCode::OK;
    type SuccessResponse =
        Sse<FilterMap<WatchStream<VoteState>, fn(VoteState) -> Option<Result<Event, APIError>>>>;

    async fn route(request: Self::Request) -> Result<Self::SuccessResponse, APIError> {
        let VoteStateWatchRequest {
            auth,
            state: State(state),
        } = request;

        let upon_event = |new_state: VoteState| match new_state {
            VoteState::Creation => Some(Ok::<Event, APIError>(Event::default().data("Creation"))),
            VoteState::Voting => Some(Ok::<Event, APIError>(Event::default().data("Voting"))),
            VoteState::Tally => Some(Ok::<Event, APIError>(Event::default().data("Tally"))),
        };

        let meetings = state.meetings()?;

        if let Some(meeting) = meetings.lock().await.get(&auth.muuid) {
            let state_rx = meeting.vote_auth.new_state_watcher();
            let stream = WatchStream::new(state_rx).filter_map(upon_event as _);
            Ok(Sse::new(stream))
        } else {
            Err(APIError::from_error_code(APIErrorCode::MUuidNotFound))
        }
    }
}
