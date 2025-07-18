use axum::{Json, http::StatusCode, response::IntoResponse};
use serde::{Deserialize, Serialize};

use crate::AuthUser;

#[derive(Deserialize)]
pub struct AuthMeetingQuery {
    muid: String,
}

#[derive(Serialize)]
pub struct AuthResponse {
    success: bool,
}

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
            Json(serde_json::to_string(&AuthResponse { success: true }).unwrap()),
        )
    } else {
        (
            StatusCode::FORBIDDEN,
            Json(serde_json::to_string(&AuthResponse { success: false }).unwrap()),
        )
    }
}
