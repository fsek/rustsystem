use rustsystem_core::{APIError, APIErrorCode, APIHandler, Method, add_handler};
use async_trait::async_trait;
use axum::{
    Json, Router,
    extract::State,
    http::StatusCode,
};
use serde::{Deserialize, Serialize};

use crate::{AppState, MUuid, UUuid};

pub fn api_trustauth_routes() -> Router<AppState> {
    let router = Router::new();
    let router = add_handler::<VoteActive>(router);
    add_handler::<IsVoter>(router)
}

// POST /vote-active — check whether voting is currently active in a meeting
#[derive(Deserialize)]
pub struct VoteActiveRequest {
    muuid: MUuid,
}

#[derive(Serialize)]
pub struct VoteActiveResponse {
    active: bool,
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

        let meetings = {
            let guard = state.read()?;
            guard.meetings.clone()
        };

        let meetings_guard = meetings.lock().await;
        let meeting = meetings_guard
            .get(&body.muuid)
            .ok_or_else(|| APIError::from_error_code(APIErrorCode::MUuidNotFound))?;

        Ok(Json(VoteActiveResponse {
            active: meeting.vote_auth.is_active(),
        }))
    }
}

// POST /is-voter — check whether a uuuid is a valid voter in a meeting
#[derive(Deserialize)]
pub struct IsVoterRequest {
    uuuid: UUuid,
    muuid: MUuid,
}

#[derive(Serialize)]
pub struct IsVoterResponse {
    is_voter: bool,
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

        let meetings = {
            let guard = state.read()?;
            guard.meetings.clone()
        };

        let meetings_guard = meetings.lock().await;
        let meeting = meetings_guard
            .get(&body.muuid)
            .ok_or_else(|| APIError::from_error_code(APIErrorCode::MUuidNotFound))?;

        Ok(Json(IsVoterResponse {
            is_voter: meeting.voters.contains_key(&body.uuuid),
        }))
    }
}
