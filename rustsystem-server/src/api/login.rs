use api_derive::APIEndpointError;
use axum::{Json, extract::State, http::StatusCode};
use axum_extra::extract::{
    CookieJar,
    cookie::{self, Cookie},
};
use serde::Deserialize;
use tracing::{error, info};

use api_core::{APIErrorCode, APIHandler, APIResult};

use crate::{AppState, tokens::get_meeting_jwt};

#[derive(Deserialize)]
pub struct LoginRequest {
    pub uuid: String,
    pub muid: String,
}

#[derive(APIEndpointError)]
#[api(endpoint(method = "POST", path = "/login"))]
pub enum LoginError {
    #[api(code = APIErrorCode::InvalidUUID, status=400)]
    InvalidUUID,
    #[api(code = APIErrorCode::InvalidMUID, status=400)]
    InvalidMUID,

    #[api(code = APIErrorCode::UUIDNotFound, status=404)]
    UUIDNotFound,
    #[api(code = APIErrorCode::MUIDNotFound, status=404)]
    MUIDNotFound,

    #[api(code = APIErrorCode::UUIDAlreadyClaimed, status=409)]
    UUIDAlreadyClaimed,
}

/// Endpoint for logging in and claiming a UUID (voter)
///
/// Returns 202 ACCEPTED upon success
pub struct Login;
impl APIHandler for Login {
    type State = AppState;
    type Request = (CookieJar, State<AppState>, Json<LoginRequest>);

    const SUCCESS_CODE: StatusCode = StatusCode::ACCEPTED;
    type SuccessResponse = CookieJar;
    type ErrorResponse = LoginError;

    async fn route(
        request: Self::Request,
    ) -> APIResult<Self::SuccessResponse, Self::ErrorResponse> {
        let (jar, State(state), Json(body)) = request;
        let uuid = if let Ok(id) = body.uuid.parse() {
            id
        } else {
            return Err(LoginError::InvalidUUID);
        };

        let muid = if let Ok(id) = body.muid.parse() {
            id
        } else {
            return Err(LoginError::InvalidMUID);
        };

        if let Some(meeting) = state.meetings.lock().await.get_mut(&muid) {
            if let Some(voter) = meeting.voters.get_mut(&uuid) {
                if voter.logged_in {
                    // If voter has already logged in, it means that this specific
                    // uuid has already been claimed.
                    error!("Voter id {uuid} has already been claimed");
                    return Err(LoginError::UUIDAlreadyClaimed);
                } else {
                    // Claim this uuid
                    voter.logged_in = true;
                    // Login resource needs to refresh
                    meeting.invite_auth.set_state(true);
                }
            } else {
                return Err(LoginError::UUIDNotFound);
            }
        } else {
            return Err(LoginError::MUIDNotFound);
        }

        let jwt = get_meeting_jwt(uuid, muid, false, &state.secret);
        let new_cookie = Cookie::build(("access_token", jwt))
            .http_only(true)
            .secure(true)
            .same_site(cookie::SameSite::Strict)
            .path("/");

        info!("Voter with id {uuid} has been accepted");
        Ok(jar.add(new_cookie))
    }
}
