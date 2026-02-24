use async_trait::async_trait;
use axum::{Json, extract::State, http::StatusCode};
use axum_extra::extract::CookieJar;
use serde::{Deserialize, Serialize};
use tracing::{error, info};

use rustsystem_core::{APIError, APIErrorCode, APIHandler, Method};

use crate::{
    AppState,
    admin_auth::AdminCred,
    tokens::{get_meeting_jwt, new_cookie},
};

#[derive(Deserialize, Serialize)]
pub struct LoginRequest {
    pub uuuid: String,
    pub muuid: String,
    pub admin_cred: Option<AdminCred>,
}

/// Endpoint for logging in and claiming a UUID (voter)
///
/// Returns 202 ACCEPTED upon success
pub struct Login;
#[async_trait]
impl APIHandler for Login {
    type State = AppState;
    type Request = (CookieJar, State<AppState>, Json<LoginRequest>);
    type SuccessResponse = CookieJar;

    const METHOD: Method = Method::Post;
    const PATH: &'static str = "/login";
    const SUCCESS_CODE: StatusCode = StatusCode::ACCEPTED;

    async fn route(request: Self::Request) -> Result<Self::SuccessResponse, APIError> {
        let (jar, State(state), Json(body)) = request;
        let uuuid = if let Ok(id) = body.uuuid.parse() {
            id
        } else {
            return Err(APIError::from_error_code(APIErrorCode::InvalidUUuid));
        };

        let muuid = if let Ok(id) = body.muuid.parse() {
            id
        } else {
            return Err(APIError::from_error_code(APIErrorCode::InvalidMUuid));
        };

        let state_guard = {
            let guard = state.read()?;
            guard.clone()
        };

        if let Some(meeting) = state_guard.meetings.lock().await.get_mut(&muuid) {
            if let Some(voter) = meeting.voters.get_mut(&uuuid) {
                if voter.logged_in {
                    // If voter has already logged in, it means that this specific
                    // uuid has already been claimed.
                    error!("Voter id {uuuid} has already been claimed");
                    return Err(APIError::from_error_code(APIErrorCode::UUIDAlreadyClaimed));
                } else {
                    // Claim this uuid
                    voter.logged_in = true;
                    // Login resource needs to refresh
                    meeting.invite_auth.set_state(true);
                }
            } else {
                return Err(APIError::from_error_code(APIErrorCode::UUuidNotFound));
            }

            let is_host = if let Some(admin_cred) = body.admin_cred {
                info!(
                    "Received admin credentials - msg length: {}, sig: {}",
                    admin_cred.get_msg().len(),
                    admin_cred.get_sig_str()
                );
                let is_valid = meeting.admin_auth.validate_token(admin_cred);
                info!("Admin credential validation result: {}", is_valid);
                is_valid
            } else {
                info!("No admin credentials provided");
                false
            };

            info!("Creating JWT with is_host: {}", is_host);
            let jwt = match get_meeting_jwt(uuuid, muuid, is_host, &state_guard.secret) {
                Ok(token) => token,
                Err(e) => {
                    error!("{e}");
                    return Err(APIError::from_error_code(APIErrorCode::Other));
                }
            };
            let new_cookie = new_cookie(jwt, state_guard.is_secure);

            info!("Voter with id {uuuid} has been accepted");
            Ok(jar.add(new_cookie))
        } else {
            Err(APIError::from_error_code(APIErrorCode::MUuidNotFound))
        }
    }
}
