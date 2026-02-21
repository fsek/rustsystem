use api_core::{APIErrorCode, APIHandler, APIResult};
use api_derive::APIEndpointError;
use axum::{
    Json,
    extract::{FromRequest, State},
    http::StatusCode,
};
use serde::Deserialize;
use zkryptium::schemes::{algorithms::BbsBls12381Sha256, generics::BlindSignature};

use crate::{AppState, tokens::AuthUser};

// ─── IsRegistered ─────────────────────────────────────────────────────────────

#[derive(FromRequest)]
pub struct IsRegisteredRequest {
    auth: AuthUser,
    state: State<AppState>,
}

#[derive(APIEndpointError)]
#[api(endpoint(method = "GET", path = "/api/voter/is-registered"))]
pub enum IsRegisteredError {
    #[api(code = APIErrorCode::MUuidNotFound, status = 404)]
    MUIDNotFound,
}

pub struct IsRegistered;
impl APIHandler for IsRegistered {
    type State = AppState;
    type Request = IsRegisteredRequest;

    const SUCCESS_CODE: StatusCode = StatusCode::OK;
    type SuccessResponse = Json<bool>;
    type ErrorResponse = IsRegisteredError;

    async fn route(
        request: Self::Request,
    ) -> APIResult<Self::SuccessResponse, Self::ErrorResponse> {
        let IsRegisteredRequest { auth, state } = request;
        let meetings = state.meetings.lock().await;
        let meeting = meetings
            .get(&auth.muuid)
            .ok_or(IsRegisteredError::MUIDNotFound)?;
        let registered = meeting
            .vote_auth
            .round_ref()
            .map(|r| r.is_registered(auth.uuuid))
            .unwrap_or(false);
        Ok(Json(registered))
    }
}

// ─── IsSubmitted ──────────────────────────────────────────────────────────────

#[derive(Deserialize)]
pub struct IsSubmittedRequest {
    pub signature: BlindSignature<BbsBls12381Sha256>,
}

#[derive(APIEndpointError)]
#[api(endpoint(method = "POST", path = "/api/voter/is-submitted"))]
pub enum IsSubmittedError {
    #[api(code = APIErrorCode::MUuidNotFound, status = 404)]
    MUIDNotFound,
}

pub struct IsSubmitted;
impl APIHandler for IsSubmitted {
    type State = AppState;
    type Request = (AuthUser, State<AppState>, Json<IsSubmittedRequest>);

    const SUCCESS_CODE: StatusCode = StatusCode::OK;

    type SuccessResponse = Json<bool>;
    type ErrorResponse = IsSubmittedError;

    async fn route(
        request: Self::Request,
    ) -> APIResult<Self::SuccessResponse, Self::ErrorResponse> {
        let (auth, State(state), Json(body)) = request;
        let meetings = state.meetings.lock().await;
        let meeting = meetings
            .get(&auth.muuid)
            .ok_or(IsSubmittedError::MUIDNotFound)?;
        let submitted = meeting
            .vote_auth
            .round_ref()
            .map(|r| r.is_used(&body.signature))
            .unwrap_or(false);
        Ok(Json(submitted))
    }
}
