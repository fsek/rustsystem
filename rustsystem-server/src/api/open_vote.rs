use std::io;

use axum::{
    Json,
    extract::State,
    http::StatusCode,
    response::{IntoResponse, Response, Sse, sse::Event},
};
use serde::{Deserialize, Serialize};
use tokio_stream::{StreamExt, wrappers::WatchStream};
use tracing::info;

use crate::{AppState, tokens::AuthUser};

#[derive(Serialize)]
pub struct IsActiveResponse {
    isActive: bool,
}

pub async fn is_active(
    AuthUser {
        uuid,
        muid,
        is_host,
    }: AuthUser,
    State(state): State<AppState>,
) -> Response {
    let res = if let Some(meeting) = state.meetings.lock().await.get(&muid) {
        meeting.vote_auth.is_active()
    } else {
        return StatusCode::NOT_FOUND.into_response();
    };

    (StatusCode::OK, Json(IsActiveResponse { isActive: res })).into_response()
}

#[derive(Serialize)]
enum StateChange {
    Start,
    Stop,
}

#[derive(Serialize)]
struct StateWatchResponse {
    change: StateChange,
}

pub async fn sse_watch_state(
    AuthUser {
        uuid,
        muid,
        is_host,
    }: AuthUser,
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

#[derive(Deserialize)]
pub struct StartVoteRequest {
    name: String,
}

pub async fn start_vote(
    AuthUser {
        uuid,
        muid,
        is_host,
    }: AuthUser,
    State(state): State<AppState>,
    Json(body): Json<StartVoteRequest>,
) -> Response {
    if let Some(meeting) = state.meetings.lock().await.get_mut(&muid) {
        if !is_host {
            return StatusCode::UNAUTHORIZED.into_response();
        } else {
            info!("Starting vote: {}", body.name);
            meeting.get_auth().set_active_state(true);
            return StatusCode::OK.into_response();
        }
    } else {
        return StatusCode::NOT_FOUND.into_response();
    }
}

pub async fn stop_vote(
    AuthUser {
        uuid,
        muid,
        is_host,
    }: AuthUser,
    State(state): State<AppState>,
) -> Response {
    if let Some(meeting) = state.meetings.lock().await.get_mut(&muid) {
        if !is_host {
            return StatusCode::UNAUTHORIZED.into_response();
        } else {
            let vote_auth = meeting.get_auth();
            vote_auth.set_active_state(false);
            vote_auth.reset();

            return StatusCode::OK.into_response();
        }
    } else {
        return StatusCode::NOT_FOUND.into_response();
    }
}
