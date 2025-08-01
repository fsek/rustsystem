use std::io;

use axum::{
    extract::State,
    http::StatusCode,
    response::{IntoResponse, Response, Sse, sse::Event},
};
use tokio_stream::{StreamExt, wrappers::WatchStream};

use crate::AppState;

use super::auth::AuthVoter;

pub async fn sse_watch_state(
    AuthVoter { uuid, muid }: AuthVoter,
    State(state): State<AppState>,
) -> Response {
    if let Some(meeting) = state.meetings.lock().await.get(&muid) {
        let state_rx = meeting.vote_auth.new_watcher();
        let stream = WatchStream::new(state_rx).filter_map(|new_state| {
            if new_state {
                Some(Ok::<Event, io::Error>(Event::default().data("Start")))
            } else {
                Some(Ok::<Event, io::Error>(Event::default().data("Stop")))
            }
        });
        return (StatusCode::OK, Sse::new(stream)).into_response();
    } else {
        return StatusCode::NOT_FOUND.into_response();
    }
}
