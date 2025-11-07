use api_derive::APIEndpointError;
use axum::{Json, http::StatusCode};
use serde::{Deserialize, Serialize};

use api_core::{APIErrorCode, APIHandler, APIResult};

use crate::{AppState, AuthUser, UUuid};

#[derive(Deserialize)]
pub struct AuthMeetingRequest {
    muuid: String,
}

#[derive(Serialize)]
pub struct AuthResponse {
    uuuid: String,
    muuid: String,
    is_host: bool,
}

#[derive(APIEndpointError)]
#[api(endpoint(method = "POST", path = "/api/auth-meeting"))]
pub enum AuthMeetingError {
    #[api(code = APIErrorCode::InvalidUUuid, status = 400)]
    InvalidMUuid,
    #[api(code = APIErrorCode::InvalidUUuid, status = 400)]
    MUIDMismatch,
}

/// Endpoint for checking if the current user is authenticated for a given meeting
///
/// Returns 200 OK upon success
pub struct AuthMeeting;
impl APIHandler for AuthMeeting {
    type State = AppState;
    type Request = (AuthUser, Json<AuthMeetingRequest>);

    const SUCCESS_CODE: StatusCode = StatusCode::OK;
    type SuccessResponse = Json<AuthResponse>;
    type ErrorResponse = AuthMeetingError;

    async fn route(
        request: Self::Request,
    ) -> APIResult<Self::SuccessResponse, Self::ErrorResponse> {
        let (
            AuthUser {
                uuuid,
                muuid,
                is_host,
            },
            Json(body),
        ) = request;
        let parsed_muid = if let Ok(parsed) = UUuid::parse_str(&body.muuid) {
            parsed
        } else {
            return Err(AuthMeetingError::InvalidMUuid);
        };
        if muuid == parsed_muid {
            Ok(Json(AuthResponse {
                uuuid: uuuid.to_string(),
                muuid: muuid.to_string(),
                is_host,
            }))
        } else {
            Err(AuthMeetingError::MUIDMismatch)
        }
    }
}
