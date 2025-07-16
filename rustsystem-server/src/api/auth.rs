use axum::{Json, http::StatusCode, response::IntoResponse};
use serde::Deserialize;

use crate::{AuthUser, MUID};

#[derive(Deserialize)]
pub struct AuthMeetingQuery {
    muid: String,
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
            Json(format!(
                "Hello user with ID: {uuid}. You are logged into meeing with muid {muid}. You are {}the meeting host",
                if is_host { "" } else { "not " }
            )),
        )
    } else {
        (
            StatusCode::FORBIDDEN,
            Json(format!(
                "Your token does not permit entry in meeting {parsed_muid}"
            )),
        )
    }
}
