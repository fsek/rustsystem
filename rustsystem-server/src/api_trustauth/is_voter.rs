use rustsystem_core::{APIError, APIHandler, Method};
use async_trait::async_trait;
use axum::{
    Json,
    extract::State,
    http::StatusCode,
};
use serde::{Deserialize, Serialize};

use crate::{AppState, MUuid, UUuid};

// POST /is-voter — check whether a uuuid is a valid voter in a meeting
#[derive(Deserialize)]
pub struct IsVoterRequest {
    pub uuuid: UUuid,
    pub muuid: MUuid,
}

#[derive(Serialize)]
pub struct IsVoterResponse {
    pub is_voter: bool,
}

pub struct IsVoter;

#[async_trait]
impl APIHandler for IsVoter {
    type State = AppState;
    type Request = (State<AppState>, Json<IsVoterRequest>);
    type SuccessResponse = Json<IsVoterResponse>;

    const METHOD: Method = Method::Post;
    const PATH: &'static str = "/is-voter";
    const SUCCESS_CODE: StatusCode = StatusCode::OK;

    async fn route(request: Self::Request) -> Result<Self::SuccessResponse, APIError> {
        let (State(state), Json(body)) = request;

        let meeting = state.get_meeting(body.muuid).await?;
        let is_voter = meeting.voters.read().await.contains_key(&body.uuuid);

        Ok(Json(IsVoterResponse { is_voter }))
    }
}
