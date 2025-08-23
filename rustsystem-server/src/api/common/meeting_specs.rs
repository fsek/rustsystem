use axum::{
    Json,
    extract::{FromRequest, State},
    http::StatusCode,
};
use serde::Serialize;

use api_core::{APIHandler, APIResponse};

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

#[derive(Serialize)]
pub enum MeetingSpecsError {
    MUIDNotFound,
}

pub struct MeetingSpecs;

impl APIHandler for MeetingSpecs {
    type State = AppState;
    type Request = MeetingSpecsRequest;
    type SuccessResponse = Json<MeetingSpecsResponse>;
    type ErrorResponse = Json<MeetingSpecsError>;

    async fn handler(
        request: Self::Request,
    ) -> APIResponse<Self::SuccessResponse, Self::ErrorResponse> {
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
            Ok((
                StatusCode::OK,
                Json(MeetingSpecsResponse {
                    title: meeting.title.clone(),
                    participants: meeting.voters.len(),
                }),
            ))
        } else {
            Err((StatusCode::NOT_FOUND, Json(MeetingSpecsError::MUIDNotFound)))
        }
    }
}
