use rustsystem_core::{APIError, APIHandler, Method};
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

        let meeting = state.get_meeting(auth.muuid).await?;
        let vote_auth = meeting.vote_auth.read().await;
        let submitted = vote_auth
            .round_ref()
            .map(|r| r.is_used(&body.signature))
            .unwrap_or(false);

        Ok(Json(submitted))
    }
}
