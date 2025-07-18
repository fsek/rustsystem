use axum::{Json, http::StatusCode, response::IntoResponse};
use serde::{Deserialize, Serialize};

use crate::{AuthUser, MUID, UUID};

#[derive(Deserialize)]
pub struct AuthMeetingQuery {
    muid: String,
}

#[derive(Serialize)]
pub struct AuthResponse {
    uuid: Option<String>,
    muid: Option<String>,
    is_host: Option<bool>,
    success: bool,
}
impl AuthResponse {
    pub fn json_success(uuid: UUID, muid: MUID, is_host: bool) -> Json<String> {
        Json(
            serde_json::to_string(&Self {
                uuid: Some(uuid.to_string()),
                muid: Some(muid.to_string()),
                is_host: Some(is_host),
                success: true,
            })
            .unwrap(),
        )
    }
    pub fn json_fail() -> Json<String> {
        Json(
            serde_json::to_string(&Self {
                uuid: None,
                muid: None,
                is_host: None,
                success: false,
            })
            .unwrap(),
        )
    }
}

/// Endpoint for checking if the current user is authenticated for a given meeting
pub async fn auth_meeting(
    AuthUser {
        uuid,
        muid,
        is_host,
    }: AuthUser,
    Json(query): Json<AuthMeetingQuery>,
) -> impl IntoResponse {
    let parsed_muid = if let Ok(parsed) = query.muid.parse::<u128>() {
        parsed
    } else {
        return (StatusCode::FORBIDDEN, Json(String::from("Invalid muid")));
    };
    if muid == parsed_muid {
        (
            StatusCode::OK,
            AuthResponse::json_success(uuid, muid, is_host),
        )
    } else {
        (StatusCode::FORBIDDEN, AuthResponse::json_fail())
    }
}
