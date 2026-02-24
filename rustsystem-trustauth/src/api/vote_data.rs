use api_core::{APIError, APIErrorCode, APIHandler, Method};
use async_trait::async_trait;
use axum::{
    Json,
    extract::{FromRequest, State},
    http::StatusCode,
};
use serde::Serialize;

use crate::{AppState, tokens::TrustAuthUser};

#[derive(FromRequest)]
pub struct GetVoteDataRequest {
    auth: TrustAuthUser,
    state: State<AppState>,
}

#[derive(Serialize)]
pub struct GetVoteDataResponse {
    pub token: Vec<u8>,
    pub blind_factor: Vec<u8>,
    pub signature: serde_json::Value,
}

pub struct GetVoteData;
#[async_trait]
impl APIHandler for GetVoteData {
    type State = AppState;
    type Request = GetVoteDataRequest;
    type SuccessResponse = Json<GetVoteDataResponse>;

    const METHOD: Method = Method::Get;
    const PATH: &'static str = "/vote-data";
    const SUCCESS_CODE: StatusCode = StatusCode::OK;

    async fn route(request: Self::Request) -> Result<Self::SuccessResponse, APIError> {
        let GetVoteDataRequest { auth, state } = request;

        if !state.vote_active(auth.muuid).await? {
            return Err(APIError::from_error_code(APIErrorCode::VotingInactive));
        }

        let rounds = state.rounds();
        let rounds_lock = rounds.lock().await;
        let round = rounds_lock
            .get(&auth.muuid)
            .ok_or_else(|| APIError::from_error_code(APIErrorCode::MUuidNotFound))?;

        let reg = round
            .registered_voters
            .get(&auth.uuuid)
            .ok_or_else(|| APIError::from_error_code(APIErrorCode::NotRegistered))?;

        Ok(Json(GetVoteDataResponse {
            token: reg.token.clone(),
            blind_factor: reg.blind_factor.clone(),
            signature: reg.signature.clone(),
        }))
    }
}
