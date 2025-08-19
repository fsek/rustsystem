use axum::{Json, extract::State, http::StatusCode};
use axum_extra::extract::{
    CookieJar,
    cookie::{self, Cookie},
};
use serde::{Deserialize, Serialize};
use tracing::{error, info};

use crate::{AppState, tokens::get_meeting_jwt};

use super::APIHandler;

#[derive(Deserialize)]
pub struct LoginRequest {
    pub uuid: String,
    pub muid: String,
}

#[derive(Serialize)]
pub enum LoginError {
    InvalidUUID,
    InvalidMUID,

    UUIDAlreadyClaimed,
    UUIDNotFound,
    MUIDNotFound,
}

/// Endpoint for logging in and claiming a UUID (voter)
///
/// Returns 202 ACCEPTED upon success
pub struct Login;
impl APIHandler for Login {
    type State = AppState;
    type Request = (CookieJar, State<AppState>, Json<LoginRequest>);
    type SuccessResponse = CookieJar;
    type ErrorResponse = Json<LoginError>;

    async fn handler(
        request: Self::Request,
    ) -> super::APIResponse<Self::SuccessResponse, Self::ErrorResponse> {
        let (jar, State(state), Json(body)) = request;
        let uuid = if let Ok(id) = body.uuid.parse() {
            id
        } else {
            return Err((StatusCode::BAD_REQUEST, Json(LoginError::InvalidUUID)));
        };

        let muid = if let Ok(id) = body.muid.parse() {
            id
        } else {
            return Err((StatusCode::BAD_REQUEST, Json(LoginError::InvalidMUID)));
        };

        if let Some(meeting) = state.meetings.lock().await.get_mut(&muid) {
            if let Some(voter) = meeting.voters.get_mut(&uuid) {
                if voter.logged_in {
                    // If voter has already logged in, it means that this specific
                    // uuid has already been claimed.
                    error!("Voter id {uuid} has already been claimed");
                    return Err((StatusCode::FORBIDDEN, Json(LoginError::UUIDAlreadyClaimed)));
                } else {
                    // Claim this uuid
                    voter.logged_in = true;
                    // Login resource needs to refresh
                    meeting.invite_auth.set_state(true);
                }
            } else {
                return Err((StatusCode::NOT_FOUND, Json(LoginError::UUIDNotFound)));
            }
        } else {
            return Err((StatusCode::NOT_FOUND, Json(LoginError::MUIDNotFound)));
        }

        let jwt = get_meeting_jwt(uuid, muid, false, &state.secret);
        let new_cookie = Cookie::build(("access_token", jwt))
            .http_only(true)
            .secure(true)
            .same_site(cookie::SameSite::Strict)
            .path("/");

        info!("Voter with id {uuid} has been accepted");
        Ok((StatusCode::ACCEPTED, jar.add(new_cookie)))
    }
}
