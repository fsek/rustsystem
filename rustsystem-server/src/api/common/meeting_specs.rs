use api_derive::APIEndpointError;
use axum::{
    Json,
    extract::{FromRequest, State},
    http::StatusCode,
};
use serde::Serialize;

use api_core::{APIErrorCode, APIHandler, APIResult};

use crate::{AppState, tokens::AuthUser};

#[derive(FromRequest)]
pub struct MeetingSpecsRequest {
    auth: AuthUser,
    state: State<AppState>,
}

#[derive(Serialize)]
pub struct MeetingSpecsResponse {
    title: String,
    participants: usize,
}

#[derive(APIEndpointError)]
#[api(endpoint(method = "GET", path = "api/common/meeting-specs"))]
pub enum MeetingSpecsError {
    #[api(code = APIErrorCode::MUIDNotFound, status = 404)]
    MUIDNotFound,
}

pub struct MeetingSpecs;

impl APIHandler for MeetingSpecs {
    type State = AppState;
    type Request = MeetingSpecsRequest;

    const SUCCESS_CODE: StatusCode = StatusCode::OK;
    type SuccessResponse = Json<MeetingSpecsResponse>;
    type ErrorResponse = MeetingSpecsError;

    async fn route(
        request: Self::Request,
    ) -> APIResult<Self::SuccessResponse, Self::ErrorResponse> {
        let MeetingSpecsRequest {
            auth:
                AuthUser {
                    uuid,
                    muid,
                    is_host,
                },
            state: State(state),
        } = request;

        if let Some(meeting) = state.meetings.lock().await.get(&muid) {
            Ok(Json(MeetingSpecsResponse {
                title: meeting.title.clone(),
                participants: meeting.voters.len(),
            }))
        } else {
            Err(MeetingSpecsError::MUIDNotFound)
        }
    }
}
