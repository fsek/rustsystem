use api_derive::APIEndpointError;
use axum::{Json, extract::State, http::StatusCode};
use axum_extra::extract::CookieJar;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tracing::{error, info};

use crate::{
    AppState, MUuid, UUuid, Voter,
    admin_auth::AdminAuthority,
    invite_auth::InviteAuthority,
    tokens::{new_cookie, new_meeting_jwt},
    vote_auth::VoteAuthority,
};

use api_core::{APIErrorCode, APIHandler, APIResult};

#[derive(Deserialize, Serialize)]
pub struct CreateMeetingRequest {
    pub title: String,
    pub host_name: String,
}

#[derive(Serialize, Deserialize)]
pub struct CreateMeetingResponse {
    pub muuid: UUuid,
    pub uuuid: MUuid,
}

#[derive(APIEndpointError)]
#[api(endpoint(method = "POST", path = "/create-meeting"))]
pub enum CreateMeetingError {
    #[api(code = APIErrorCode::Other, status=500)]
    Other,
}

/// Endpoint for creating a new meeting resource
///
/// Returns 201 CREATED upon success
pub struct CreateMeeting;
impl APIHandler for CreateMeeting {
    type State = AppState;
    type Request = (CookieJar, State<AppState>, Json<CreateMeetingRequest>);

    const SUCCESS_CODE: StatusCode = StatusCode::CREATED;
    type SuccessResponse = (CookieJar, Json<CreateMeetingResponse>);
    type ErrorResponse = CreateMeetingError;
    async fn route(
        request: Self::Request,
    ) -> APIResult<Self::SuccessResponse, Self::ErrorResponse> {
        let (jar, State(state), Json(query)) = request;

        let (uuuid, muuid, jwt) = match new_meeting_jwt(&state.secret) {
            Ok(res) => res,
            Err(e) => {
                error!("{e}");
                return Err(CreateMeetingError::Other);
            }
        };
        let new_cookie = new_cookie(jwt, state.is_secure);

        info!("Creating new meeting with id {muuid} and host {uuuid}");
        let mut meetings = state.meetings.lock().await;
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
                agenda: String::new(),
                voters,
                vote_auth,
                invite_auth,
                admin_auth: AdminAuthority::new(),
                locked: false,
            },
        );

        Ok((
            jar.add(new_cookie),
            Json(CreateMeetingResponse { muuid, uuuid }),
        ))
    }
}
