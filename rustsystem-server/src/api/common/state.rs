use axum::{
    Json,
    extract::State,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use serde::Serialize;

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
