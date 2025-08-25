use api_derive::APIEndpointError;
use axum::{Json, http::StatusCode};
use serde::{Deserialize, Serialize};

use api_core::{APIErrorCode, APIHandler, APIResult};

use crate::{AppState, AuthUser};

#[derive(Deserialize)]
pub struct AuthMeetingRequest {
    muid: String,
}

#[derive(Serialize)]
pub struct AuthResponse {
    uuid: String,
    muid: String,
    is_host: bool,
}

#[derive(APIEndpointError)]
#[api(endpoint(method = "POST", path = "/api/auth-meeting"))]
pub enum AuthMeetingError {
    #[api(code = APIErrorCode::InvalidUUID, status = 400)]
    InvalidMUID,
    #[api(code = APIErrorCode::InvalidUUID, status = 400)]
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
                uuid,
                muid,
                is_host,
            },
            Json(body),
        ) = request;
        let parsed_muid = if let Ok(parsed) = body.muid.parse::<u128>() {
            parsed
        } else {
            return Err(AuthMeetingError::InvalidMUID);
        };
        if muid == parsed_muid {
            Ok(Json(AuthResponse {
                uuid: uuid.to_string(),
                muid: muid.to_string(),
                is_host,
            }))
        } else {
            Err(AuthMeetingError::MUIDMismatch)
        }
    }
}
