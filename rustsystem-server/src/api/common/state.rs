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
pub struct VoteActiveRequest {
    auth: AuthUser,
    state: State<AppState>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VoteActiveResponse {
    is_active: bool,
}

#[derive(APIEndpointError)]
#[api(endpoint(method = "GET", path = "/api/common/vote-active"))]
pub enum VoteActiveError {
    #[api(code = APIErrorCode::MUIDNotFound, status = 404)]
    MUIDNotFound,
}

pub struct VoteActive;
impl APIHandler for VoteActive {
    type State = AppState;
    type Request = VoteActiveRequest;

    const SUCCESS_CODE: StatusCode = StatusCode::OK;
    type SuccessResponse = Json<VoteActiveResponse>;
    type ErrorResponse = VoteActiveError;
    async fn route(
        request: Self::Request,
    ) -> APIResult<Self::SuccessResponse, Self::ErrorResponse> {
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
            return Err(VoteActiveError::MUIDNotFound);
        };

        Ok(Json(VoteActiveResponse { is_active: res }))
    }
}
