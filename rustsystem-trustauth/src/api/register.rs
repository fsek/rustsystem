use rustsystem_core::{APIError, APIErrorCode, APIHandler, Method};
use async_trait::async_trait;
use axum::{
    Json,
    extract::{FromRequest, State},
    http::StatusCode,
};
use serde::{Deserialize, Serialize};
use zkryptium::schemes::{
    algorithms::BbsBls12381Sha256,
    generics::{BlindSignature, Commitment},
};

use crate::{AppState, VoterRegistration, tokens::TrustAuthUser};

#[derive(Deserialize)]
pub struct RegisterRequest {
    pub commitment: Commitment<BbsBls12381Sha256>,
    pub token: Vec<u8>,
    pub blind_factor: Vec<u8>,
    pub context: serde_json::Value,
}

pub struct Register;
#[async_trait]
impl APIHandler for Register {
    type State = AppState;
    type Request = (TrustAuthUser, State<AppState>, Json<RegisterRequest>);
    type SuccessResponse = ();

    const METHOD: Method = Method::Post;
    const PATH: &'static str = "/register";
    const SUCCESS_CODE: StatusCode = StatusCode::CREATED;

    async fn route(request: Self::Request) -> Result<Self::SuccessResponse, APIError> {
        let (auth, State(state), Json(body)) = request;

        if !state.vote_active(auth.muuid).await? {
            return Err(APIError::from_error_code(APIErrorCode::VotingInactive));
        }

        if !state.is_voter(auth.uuuid, auth.muuid).await? {
            return Err(APIError::from_error_code(APIErrorCode::UUuidNotFound));
        }

        let rounds = state.rounds();
        let mut rounds_lock = rounds.lock().await;
        let round = rounds_lock
            .get_mut(&auth.muuid)
            .ok_or_else(|| APIError::from_error_code(APIErrorCode::MUuidNotFound))?;

        if round.registered_voters.contains_key(&auth.uuuid) {
            return Err(APIError::from_error_code(APIErrorCode::AlreadyRegistered));
        }

        let signature = BlindSignature::<BbsBls12381Sha256>::blind_sign(
            round.keys.private_key(),
            round.keys.public_key(),
            Some(&body.commitment.to_bytes()),
            Some(&round.header),
            None,
        )
        .map_err(|_| APIError::from_error_code(APIErrorCode::SignatureFailure))?;

        let signature_json = serde_json::to_value(&signature)
            .map_err(|_| APIError::from_error_code(APIErrorCode::Other))?;

        round.registered_voters.insert(
            auth.uuuid,
            VoterRegistration {
                token: body.token,
                blind_factor: body.blind_factor,
                commitment: body.commitment.to_bytes().to_vec(),
                context: body.context,
                signature: signature_json,
            },
        );

        Ok(())
    }
}

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
