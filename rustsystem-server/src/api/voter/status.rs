use api_core::{APIError, APIErrorCode, APIHandler, Method};
use async_trait::async_trait;
use axum::{
    Json,
    extract::{State},
    http::StatusCode,
};
use serde::Deserialize;
use zkryptium::schemes::{algorithms::BbsBls12381Sha256, generics::BlindSignature};

use crate::{AppState, tokens::AuthUser};

// ─── IsSubmitted ──────────────────────────────────────────────────────────────

#[derive(Deserialize)]
pub struct IsSubmittedRequest {
    pub signature: BlindSignature<BbsBls12381Sha256>,
}

pub struct IsSubmitted;
#[async_trait]
impl APIHandler for IsSubmitted {
    type State = AppState;
    type Request = (AuthUser, State<AppState>, Json<IsSubmittedRequest>);
    type SuccessResponse = Json<bool>;

    const METHOD: Method = Method::Post;
    const PATH: &'static str = "/is-submitted";
    const SUCCESS_CODE: StatusCode = StatusCode::OK;

    async fn route(request: Self::Request) -> Result<Self::SuccessResponse, APIError> {
        let (auth, State(state), Json(body)) = request;
        let meetings_guard = {
            let guard = state.read()?;
            guard.clone().meetings
        };

        let meetings = meetings_guard.lock().await;
        let meeting = meetings
            .get(&auth.muuid)
            .ok_or(APIError::from_error_code(APIErrorCode::MUuidNotFound))?;
        let submitted = meeting
            .vote_auth
            .round_ref()
            .map(|r| r.is_used(&body.signature))
            .unwrap_or(false);
        Ok(Json(submitted))
    }
}
