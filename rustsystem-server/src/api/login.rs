use axum::{
    Json,
    extract::State,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use axum_extra::extract::{
    CookieJar,
    cookie::{self, Cookie},
};
use serde::{Deserialize, Serialize};
use tracing::{error, info};

use crate::{AppState, tokens::get_meeting_jwt};

#[derive(Deserialize)]
pub struct LoginBody {
    pub uuid: String,
    pub muid: String,
}

#[derive(Serialize)]
pub struct LoginResponse {
    pub success: bool,
}
impl IntoResponse for LoginResponse {
    fn into_response(self) -> Response {
        Json(serde_json::to_string(&self).unwrap()).into_response()
    }
}

pub async fn login(
    jar: CookieJar,
    State(state): State<AppState>,
    Json(body): Json<LoginBody>,
) -> Response {
    let uuid = if let Ok(id) = body.uuid.parse() {
        id
    } else {
        error!("Invalid uuid");
        return (StatusCode::FORBIDDEN, LoginResponse { success: false }).into_response();
    };

    let muid = if let Ok(id) = body.muid.parse() {
        id
    } else {
        error!("Invalid muid");
        return StatusCode::FORBIDDEN.into_response();
    };

    if let Some(meeting) = state.meetings.lock().await.get_mut(&muid) {
        if let Some(voter) = meeting.voters.get_mut(&uuid) {
            if voter.logged_in {
                // If voter has already logged in, it means that this specific
                // uuid has already been claimed.
                error!("Voter id {uuid} has already been claimed");
                return (StatusCode::FORBIDDEN, LoginResponse { success: false }).into_response();
            } else {
                // Claim this uuid
                voter.logged_in = true;
            }
        } else {
            error!("Voter with id {uuid} doesn't exist!");
            return (StatusCode::FORBIDDEN, LoginResponse { success: false }).into_response();
        }
    } else {
        error!("Meeting with id {muid} doesn't exist!");
        return (StatusCode::FORBIDDEN, LoginResponse { success: false }).into_response();
    }

    let jwt = get_meeting_jwt(uuid, muid, false, &state.secret);
    let new_cookie = Cookie::build(("access_token", jwt))
        .http_only(true)
        .secure(true)
        .same_site(cookie::SameSite::Strict)
        .path("/");

    info!("Voter with id {uuid} has been accepted");
    (
        StatusCode::ACCEPTED,
        jar.add(new_cookie),
        LoginResponse { success: true },
    )
        .into_response()
}
