use rustsystem_core::{APIError, APIHandler, Method};
use async_trait::async_trait;
use axum::{
    Json,
    extract::State,
    http::StatusCode,
};
use serde::{Deserialize, Serialize};

use crate::{AppState, MUuid};

// POST /vote-active — check whether voting is currently active in a meeting
#[derive(Deserialize)]
pub struct VoteActiveRequest {
    pub muuid: MUuid,
}

#[derive(Serialize)]
pub struct VoteActiveResponse {
    pub active: bool,
}

pub struct VoteActive;

#[async_trait]
impl APIHandler for VoteActive {
    type State = AppState;
    type Request = (State<AppState>, Json<VoteActiveRequest>);
    type SuccessResponse = Json<VoteActiveResponse>;

    const METHOD: Method = Method::Post;
    const PATH: &'static str = "/vote-active";
    const SUCCESS_CODE: StatusCode = StatusCode::OK;

    async fn route(request: Self::Request) -> Result<Self::SuccessResponse, APIError> {
        let (State(state), Json(body)) = request;

        let meeting = state.get_meeting(body.muuid).await?;
        let active = meeting.vote_auth.read().await.is_active();

        Ok(Json(VoteActiveResponse { active }))
    }
}
