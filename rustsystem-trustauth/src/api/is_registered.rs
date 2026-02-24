use rustsystem_core::{APIError, APIErrorCode, APIHandler, Method};
use async_trait::async_trait;
use axum::{
    Json,
    extract::{FromRequest, State},
    http::StatusCode,
};
use serde::Serialize;

use crate::{AppState, tokens::TrustAuthUser};

#[derive(FromRequest)]
pub struct IsRegisteredRequest {
    auth: TrustAuthUser,
    state: State<AppState>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct IsRegisteredResponse {
    is_registered: bool,
}

pub struct IsRegistered;
#[async_trait]
impl APIHandler for IsRegistered {
    type State = AppState;
    type Request = IsRegisteredRequest;
    type SuccessResponse = Json<IsRegisteredResponse>;

    const METHOD: Method = Method::Get;
    const PATH: &'static str = "/is-registered";
    const SUCCESS_CODE: StatusCode = StatusCode::OK;

    async fn route(request: Self::Request) -> Result<Self::SuccessResponse, APIError> {
        let IsRegisteredRequest { auth, state } = request;

        let mut is_registered = true;

        if !state.vote_active(auth.muuid).await? {
            is_registered = false;
        }

        if !state.is_voter(auth.uuuid, auth.muuid).await? {
            is_registered = false;
        }

        let rounds = state.rounds();
        let mut rounds_lock = rounds.lock().await;
        let round = rounds_lock
            .get_mut(&auth.muuid)
            .ok_or_else(|| APIError::from_error_code(APIErrorCode::MUuidNotFound))?;
        if !round.registered_voters.contains_key(&auth.uuuid) {
            is_registered = false;
        }

        Ok(Json(IsRegisteredResponse { is_registered }))
    }
}
