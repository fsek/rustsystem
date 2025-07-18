use std::{collections::HashMap, time::SystemTime};

use axum::{Json, extract::State, http::StatusCode, response::IntoResponse};
use axum_extra::extract::{
    CookieJar,
    cookie::{self, Cookie},
};
use serde::{Deserialize, Serialize};

use crate::{AppState, Voter, tokens::new_meeting_jwt};

#[derive(Deserialize)]
pub struct CreateMeetingQuery {
    pub title: String,
}

#[derive(Serialize)]
pub struct CreateMeetingResponse {
    pub muid: String,
}

#[axum::debug_handler]
pub async fn create_meeting(
    jar: CookieJar,
    State(state): State<AppState>,
    Json(query): Json<CreateMeetingQuery>,
) -> impl IntoResponse {
    let (uuid, muid, jwt) = new_meeting_jwt(&state.secret);
    let new_cookie = Cookie::build(("access_token", jwt))
        .http_only(true)
        .secure(true)
        .same_site(cookie::SameSite::Strict)
        .path("/");

    let mut meetings = state.meetings.lock().await;
    let mut voters = HashMap::new();
    voters.insert(uuid, Voter { logged_in: true });
    meetings.insert(
        muid,
        crate::Meeting {
            host: uuid,
            title: query.title,
            start_time: SystemTime::now(),
            voters,
        },
    );
    println!("host is now {uuid}");

    (
        StatusCode::CREATED,
        jar.add(new_cookie),
        Json(CreateMeetingResponse {
            muid: muid.to_string(),
        }),
    )
}
