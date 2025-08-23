use axum::{Json, http::StatusCode, response::IntoResponse};
use serde::{Deserialize, Serialize};

use api_core::{APIHandler, APIResponse};

use crate::{AppState, AuthUser, MUID, UUID};

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

#[derive(Serialize)]
pub enum AuthMeetingError {
    InvalidMUID,
    MUIDMismatch,
}

/// Endpoint for checking if the current user is authenticated for a given meeting
///
/// Returns 200 OK upon success
pub struct AuthMeeting;
impl APIHandler for AuthMeeting {
    type State = AppState;
    type Request = (AuthUser, Json<AuthMeetingRequest>);

    type SuccessResponse = Json<AuthResponse>;
    type ErrorResponse = Json<AuthMeetingError>;

    async fn handler(
        request: Self::Request,
    ) -> APIResponse<Self::SuccessResponse, Self::ErrorResponse> {
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
            return Err((StatusCode::BAD_REQUEST, Json(AuthMeetingError::InvalidMUID)));
        };
        if muid == parsed_muid {
            Ok((
                StatusCode::OK,
                Json(AuthResponse {
                    uuid: uuid.to_string(),
                    muid: muid.to_string(),
                    is_host,
                }),
            ))
        } else {
            Err((StatusCode::NOT_FOUND, Json(AuthMeetingError::MUIDMismatch)))
        }
    }
}
