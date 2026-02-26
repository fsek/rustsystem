use async_trait::async_trait;
use axum::{Json, extract::State, http::StatusCode};
use axum_extra::extract::CookieJar;
use serde::{Deserialize, Serialize};
use std::time::SystemTime;
use std::{collections::HashMap, fs};
use tracing::{error, info};

use crate::{
    AppState, MUuid, UUuid, Voter,
    tokens::{new_cookie, new_meeting_jwt},
};

use rustsystem_core::{APIError, APIErrorCode, APIHandler, Method};

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

        let (secret, is_secure) = {
            let guard = state.read()?;
            (guard.secret, guard.is_secure)
        };

        let (uuuid, muuid, jwt) = new_meeting_jwt(&secret)?;
        let new_cookie = new_cookie(jwt, is_secure);

        info!("Creating new meeting with id {muuid} and host {uuuid}");
        let mut voters = HashMap::new();
        voters.insert(
            uuuid,
            Voter {
                name: query.host_name,
                logged_in: true,
                is_host: true,
                registered_at: SystemTime::now(),
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

        // Outer map write: inserting a new meeting and pruning stale ones.
        let meetings_arc = state.meetings_write()?;
        let mut map = meetings_arc.write().await;

        map.insert(
            muuid,
            std::sync::Arc::new(crate::Meeting::new(query.title, SystemTime::now(), voters)),
        );

        // Remove dead meetings
        let dead_muuids: Vec<_> = map
            .iter()
            .filter(|(_, m)| {
                m.start_time
                    .elapsed()
                    .unwrap_or_default()
                    .as_secs()
                    > 60 * 60 * 12
            })
            .map(|(id, _)| *id)
            .collect();
        for id in dead_muuids {
            map.remove(&id);
        }

        Ok((
            jar.add(new_cookie),
            Json(CreateMeetingResponse { muuid, uuuid }),
        ))
    }
}
