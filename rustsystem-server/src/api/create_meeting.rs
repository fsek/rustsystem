use axum::{Json, extract::State, http::StatusCode};
use axum_extra::extract::{
    CookieJar,
    cookie::{self, Cookie},
};
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, time::SystemTime};
use tracing::info;

use crate::{
    AppState, Voter, invite_auth::InviteAuthority, tokens::new_meeting_jwt,
    vote_auth::VoteAuthority,
};

use api_core::{APIHandler, APIResponse};

#[derive(Deserialize)]
pub struct CreateMeetingRequest {
    pub title: String,
}

#[derive(Serialize)]
pub struct CreateMeetingResponse {
    pub muid: String,
    pub uuid: String,
}

#[derive(Serialize)]
pub enum CreateMeetingError {}

/// Endpoint for creating a new meeting resource
///
/// Returns 201 CREATED upon success
pub struct CreateMeeting;
impl APIHandler for CreateMeeting {
    type State = AppState;
    type Request = (CookieJar, State<AppState>, Json<CreateMeetingRequest>);
    type SuccessResponse = (CookieJar, Json<CreateMeetingResponse>);
    type ErrorResponse = Json<CreateMeetingError>;
    async fn handler(
        request: Self::Request,
    ) -> APIResponse<Self::SuccessResponse, Self::ErrorResponse> {
        let (jar, State(state), Json(query)) = request;

        let (uuid, muid, jwt) = new_meeting_jwt(&state.secret);
        let new_cookie = Cookie::build(("access_token", jwt))
            .http_only(true)
            .secure(true)
            .same_site(cookie::SameSite::Strict)
            .path("/");

        info!("Creating new meeting with id {muid} and host {uuid}");
        let mut meetings = state.meetings.lock().await;
        let mut voters = HashMap::new();
        voters.insert(uuid, Voter { logged_in: true });

        let vote_auth = VoteAuthority::new();
        let invite_auth = InviteAuthority::new();

        meetings.insert(
            muid,
            crate::Meeting {
                host: uuid,
                title: query.title,
                start_time: SystemTime::now(),
                voters,
                vote_auth,
                invite_auth,
            },
        );

        Ok((
            StatusCode::CREATED,
            (
                jar.add(new_cookie),
                Json(CreateMeetingResponse {
                    muid: muid.to_string(),
                    uuid: uuid.to_string(),
                }),
            ),
        ))
    }
}
