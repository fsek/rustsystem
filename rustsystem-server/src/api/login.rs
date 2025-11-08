use api_derive::APIEndpointError;
use axum::{Json, extract::State, http::StatusCode};
use axum_extra::extract::{
    CookieJar,
    cookie::{self, Cookie},
};
use serde::{Deserialize, Serialize};
use tracing::{error, info};

use api_core::{APIErrorCode, APIHandler, APIResult};

use crate::{AppState, admin_auth::AdminCred, tokens::get_meeting_jwt};

#[derive(Deserialize, Serialize)]
pub struct LoginRequest {
    pub uuuid: String,
    pub muuid: String,
    pub admin_cred: Option<AdminCred>,
}

#[derive(APIEndpointError)]
#[api(endpoint(method = "POST", path = "/login"))]
pub enum LoginError {
    #[api(code = APIErrorCode::InvalidUUuid, status=400)]
    InvalidUUID,
    #[api(code = APIErrorCode::InvalidMUuid, status=400)]
    InvalidMUID,

    #[api(code = APIErrorCode::UUuidNotFound, status=404)]
    UUIDNotFound,
    #[api(code = APIErrorCode::MUuidNotFound, status=404)]
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
        let uuuid = if let Ok(id) = body.uuuid.parse() {
            id
        } else {
            return Err(LoginError::InvalidUUID);
        };

        let muuid = if let Ok(id) = body.muuid.parse() {
            id
        } else {
            return Err(LoginError::InvalidMUID);
        };

        if let Some(meeting) = state.meetings.lock().await.get_mut(&muuid) {
            if let Some(voter) = meeting.voters.get_mut(&uuuid) {
                if voter.logged_in {
                    // If voter has already logged in, it means that this specific
                    // uuid has already been claimed.
                    error!("Voter id {uuuid} has already been claimed");
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

            let is_host = if let Some(admin_cred) = body.admin_cred {
                meeting.admin_auth.validate_token(admin_cred)
            } else {
                false
            };

            let jwt = get_meeting_jwt(uuuid, muuid, is_host, &state.secret);
            let new_cookie = Cookie::build(("access_token", jwt))
                .http_only(true)
                .secure(true)
                .same_site(cookie::SameSite::Strict)
                .path("/");

            info!("Voter with id {uuuid} has been accepted");
            Ok(jar.add(new_cookie))
        } else {
            Err(LoginError::MUIDNotFound)
        }
    }
}
