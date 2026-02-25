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
        let uuuid = body
            .uuuid
            .parse()
            .map_err(|_| APIError::from_error_code(APIErrorCode::InvalidUUuid))?;
        let muuid = body
            .muuid
            .parse()
            .map_err(|_| APIError::from_error_code(APIErrorCode::InvalidMUuid))?;

        let meeting = state.get_meeting(muuid).await?;

        // Claim the voter slot.
        {
            let mut voters = meeting.voters.write().await;
            let voter = voters
                .get_mut(&uuuid)
                .ok_or_else(|| APIError::from_error_code(APIErrorCode::UUuidNotFound))?;
            if voter.logged_in {
                error!("Voter id {uuuid} has already been claimed");
                return Err(APIError::from_error_code(APIErrorCode::UUIDAlreadyClaimed));
            }
            voter.logged_in = true;
        } // voters write guard released

        // Signal the invite watcher.
        meeting.invite_auth.write().await.set_state(true);

        // Validate optional admin credentials.
        let is_host = if let Some(admin_cred) = body.admin_cred {
            info!(
                "Received admin credentials - msg length: {}, sig: {}",
                admin_cred.get_msg().len(),
                admin_cred.get_sig_str()
            );
            let is_valid = meeting.admin_auth.write().await.validate_token(admin_cred);
            info!("Admin credential validation result: {}", is_valid);
            is_valid
        } else {
            info!("No admin credentials provided");
            false
        };

        let (secret, is_secure) = {
            let guard = state.read()?;
            (guard.secret, guard.is_secure)
        };

        info!("Creating JWT with is_host: {}", is_host);
        let jwt = match get_meeting_jwt(uuuid, muuid, is_host, &secret) {
            Ok(token) => token,
            Err(e) => {
                error!("{e}");
                return Err(APIError::from_error_code(APIErrorCode::Other));
            }
        };
        let new_cookie = new_cookie(jwt, is_secure);

        info!("Voter with id {uuuid} has been accepted");
        Ok(jar.add(new_cookie))
    }
}
