use axum::{
    Json,
    extract::State,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use serde::Deserialize;
use tracing::info;

use crate::{AppState, tokens::AuthUser};

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
