use async_trait::async_trait;
use axum::{Json, extract::State, http::StatusCode};
use axum_extra::extract::CookieJar;
use serde::{Deserialize, Serialize};
use std::time::SystemTime;
use std::{collections::HashMap, fs};
use tracing::{error, info};

use crate::{
    AppState, MUuid, UUuid, Voter,
    admin_auth::AdminAuthority,
    invite_auth::InviteAuthority,
    tokens::{new_cookie, new_meeting_jwt},
    vote_auth::VoteAuthority,
};

use api_core::{APIError, APIErrorCode, APIHandler, Method};

#[derive(Deserialize, Serialize)]
pub struct CreateMeetingRequest {
    pub title: String,
    pub host_name: String,
    pub pub_key: String,
}

#[derive(Serialize, Deserialize)]
pub struct CreateMeetingResponse {
    pub muuid: UUuid,
    pub uuuid: MUuid,
}

/// Endpoint for creating a new meeting resource
///
/// Returns 201 CREATED upon success
pub struct CreateMeeting;
#[async_trait]
impl APIHandler for CreateMeeting {
    type State = AppState;
    type Request = (CookieJar, State<AppState>, Json<CreateMeetingRequest>);
    type SuccessResponse = (CookieJar, Json<CreateMeetingResponse>);

    const METHOD: Method = Method::Post;
    const PATH: &'static str = "/create-meeting";
    const SUCCESS_CODE: StatusCode = StatusCode::CREATED;
    async fn route(request: Self::Request) -> Result<Self::SuccessResponse, APIError> {
        let (jar, State(state), Json(query)) = request;

        let state_guard = {
            let guard = state.read()?;
            guard.clone()
        };

        let (uuuid, muuid, jwt) = match new_meeting_jwt(&state_guard.secret) {
            Ok(res) => res,
            Err(e) => {
                error!("{e}");
                return Err(APIError::from_error_code(APIErrorCode::Other));
            }
        };
        let new_cookie = new_cookie(jwt, state_guard.is_secure);

        info!("Creating new meeting with id {muuid} and host {uuuid}");
        let mut meetings = state_guard.meetings.lock().await;
        let mut voters = HashMap::new();
        voters.insert(
            uuuid,
            Voter {
                name: query.host_name,
                logged_in: true,
                is_host: true,
                registered_at: std::time::SystemTime::now(),
            },
        );

        let vote_auth = VoteAuthority::new();
        let invite_auth = InviteAuthority::new();

        meetings.insert(
            muuid,
            crate::Meeting {
                title: query.title,
                start_time: SystemTime::now(),
                agenda: String::new(),
                voters,
                vote_auth,
                invite_auth,
                admin_auth: AdminAuthority::new(),
                locked: false,
            },
        );

        // Write public key to per-meeting directory
        let meeting_dir = format!("meetings/{muuid}");
        if let Err(e) = fs::create_dir_all(&meeting_dir) {
            error!("Failed to create meeting directory {meeting_dir}: {e}");
            return Err(APIError::from_error_code(APIErrorCode::Other));
        }
        if let Err(e) = fs::write(format!("{meeting_dir}/pub_key.pem"), &query.pub_key) {
            error!("Failed to write pub_key.pem for meeting {muuid}: {e}");
            return Err(APIError::from_error_code(APIErrorCode::Other));
        }

        // Remove dead meetings
        let mut dead_muuids = Vec::new();
        for (muuid, meeting) in meetings.iter() {
            if meeting.start_time.elapsed().unwrap().as_secs() > 60 * 60 * 12 {
                dead_muuids.push(*muuid);
            }
        }
        for muuid in dead_muuids {
            meetings.remove(&muuid);
        }

        Ok((
            jar.add(new_cookie),
            Json(CreateMeetingResponse { muuid, uuuid }),
        ))
    }
}
