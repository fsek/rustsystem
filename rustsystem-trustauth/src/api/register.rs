use rustsystem_core::{APIError, APIErrorCode, APIHandler, Method};
use async_trait::async_trait;
use axum::{
    Json,
    extract::State,
    http::StatusCode,
};
use serde::Deserialize;
use tracing::info;
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

        let round = state.get_round(auth.muuid).await?;

        if round.registered_voters.read().await.contains_key(&auth.uuuid) {
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

        round.registered_voters.write().await.insert(
            auth.uuuid,
            VoterRegistration {
                token: body.token,
                blind_factor: body.blind_factor,
                commitment: body.commitment.to_bytes().to_vec(),
                context: body.context,
                signature: signature_json,
            },
        );

        info!(
            muuid = %auth.muuid,
            uuuid = %auth.uuuid,
            "Voter registered — blind signature issued"
        );

        Ok(())
    }
}
