use axum::{
    Json,
    extract::{FromRequest, State},
    http::StatusCode,
    response::{IntoResponse, Response},
};
use serde::Serialize;

use crate::{AppState, api::APIHandler, tokens::AuthUser};

#[derive(FromRequest)]
pub struct VoteActiveRequest {
    auth: AuthUser,
    state: State<AppState>,
}

#[derive(Serialize)]
pub struct VoteActiveResponse {
    isActive: bool,
}

#[derive(Serialize)]
pub enum VoteActiveError {
    MUIDNotFound,
}

pub struct VoteActive;
impl APIHandler for VoteActive {
    type State = AppState;
    type Request = VoteActiveRequest;
    type SuccessResponse = Json<VoteActiveResponse>;
    type ErrorResponse = Json<VoteActiveError>;
    async fn handler(
        request: Self::Request,
    ) -> crate::api::APIResponse<Self::SuccessResponse, Self::ErrorResponse> {
        let VoteActiveRequest {
            auth:
                AuthUser {
                    uuid,
                    muid,
                    is_host,
                },
            state: State(state),
        } = request;

        let res = if let Some(meeting) = state.meetings.lock().await.get(&muid) {
            meeting.vote_auth.is_active()
        } else {
            return Err((StatusCode::NOT_FOUND, Json(VoteActiveError::MUIDNotFound)));
        };

        Ok((StatusCode::OK, Json(VoteActiveResponse { isActive: res })))
    }
}
