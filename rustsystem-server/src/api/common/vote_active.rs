use async_trait::async_trait;
use axum::{
    Json,
    extract::{FromRequest, State},
    http::StatusCode,
};
use serde::Serialize;

use rustsystem_core::{APIError, APIHandler, Method};

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

pub struct VoteActive;
#[async_trait]
impl APIHandler for VoteActive {
    type State = AppState;
    type Request = VoteActiveRequest;

    const METHOD: Method = Method::Get;
    const PATH: &'static str = "/vote-active";
    const SUCCESS_CODE: StatusCode = StatusCode::OK;
    type SuccessResponse = Json<VoteActiveResponse>;

    async fn route(request: Self::Request) -> Result<Self::SuccessResponse, APIError> {
        let VoteActiveRequest {
            auth,
            state: State(state),
        } = request;

        let meeting = state.get_meeting(auth.muuid).await?;
        let is_active = meeting.vote_auth.read().await.is_active();

        Ok(Json(VoteActiveResponse { is_active }))
    }
}
