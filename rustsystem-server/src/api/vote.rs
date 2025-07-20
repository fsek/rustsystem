use std::collections::HashSet;

use axum::{
    Json, Router,
    extract::State,
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::post,
};
use rustsystem_proof::{
    ProofContext, Provider, RegistrationInfo, RegistrationRejectReason, RegistrationResponse,
    Sha256Provider, Sha256RegistrationInfo, Sha256ValidationInfo, ValidationInfo,
};
use serde::{Deserialize, Serialize};
use tracing::{error, info};
use zkryptium::{
    bbsplus::commitment::BlindFactor,
    keys::pair::KeyPair,
    schemes::{
        algorithms::BbsBls12381Sha256,
        generics::{BlindSignature, Commitment},
    },
};

use crate::{AppState, MUID, UUID, tokens::AuthUser};

pub fn vote_api() -> Router<AppState> {
    Router::new()
        .route("/register", post(register))
        .route("/submit", post(validate_vote))
}

#[derive(Clone)]
pub struct AuthenticationKeys(KeyPair<BbsBls12381Sha256>);

#[derive(Clone)]
pub struct Header(Vec<u8>);

pub struct VoteAuth {
    keys: AuthenticationKeys,
    header: Header,
    registered_voters: HashSet<UUID>,
}
impl VoteAuth {
    /// For new meeting
    pub fn new(header: String) -> Self {
        let keys = AuthenticationKeys(Sha256Provider::generate_authentication_keys());
        let header = Header(header.as_bytes().to_vec());
        let registered_voters = HashSet::new();

        Self {
            keys,
            header,
            registered_voters,
        }
    }

    /// Resets VoteAuth for new voting round. Old ballots are no longer valid since the
    /// keys have changed.
    /// Voters can now re-register.
    pub fn reset(&mut self) {
        self.keys.0 = Sha256Provider::generate_authentication_keys();
        self.registered_voters.clear();
    }

    pub fn is_registered(&self, uuid: UUID) -> bool {
        self.registered_voters.contains(&uuid)
    }
}

async fn register(
    AuthUser {
        uuid,
        muid,
        is_host,
    }: AuthUser,
    State(state): State<AppState>,
    Json(body): Json<Sha256RegistrationInfo>,
) -> Response {
    info!("Got register request");

    let mut meetings = state.meetings.lock().await;
    let meeting = if let Some(meeting_ok) = meetings.get_mut(&muid) {
        meeting_ok
    } else {
        return (StatusCode::NOT_FOUND, Json("Meeting could not be found")).into_response();
    };

    let vote_auth = meeting.get_auth();

    if vote_auth.is_registered(uuid) {
        return (
            StatusCode::CONFLICT,
            Json(RegistrationResponse::Rejected(
                RegistrationRejectReason::AlreadyRegistered,
            )),
        )
            .into_response();
    }

    if let Ok(signature) = Sha256Provider::sign_token(
        body.commitment,
        vote_auth.header.0.clone(),
        vote_auth.keys.0.clone(),
    ) {
        (
            StatusCode::CREATED,
            Json(RegistrationResponse::Accepted(signature)),
        )
            .into_response()
    } else {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(RegistrationRejectReason::SignatureFailure),
        )
            .into_response()
    }
}

#[derive(Deserialize)]
pub struct ValidateRequest {
    proof: Vec<u8>,
    token: Vec<u8>,
    signature: BlindSignature<BbsBls12381Sha256>,
}

async fn validate_vote(
    AuthUser {
        uuid,
        muid,
        is_host,
    }: AuthUser,
    State(state): State<AppState>,
    Json(body): Json<ValidateRequest>,
) -> Response {
    let mut meetings = state.meetings.lock().await;
    let meeting = if let Some(meeting_ok) = meetings.get_mut(&muid) {
        meeting_ok
    } else {
        return (StatusCode::NOT_FOUND, Json("Meeting could not be found")).into_response();
    };

    let vote_auth = meeting.get_auth();

    let info = Sha256ValidationInfo::new(body.proof, body.token, body.signature);

    if let Ok(_) = Sha256Provider::validate_token(
        info.get_proof(),
        vote_auth.header.0.clone(),
        info.token,
        vote_auth.keys.0.public_key().clone(),
        info.signature,
    ) {
        info!("Validation Successful");
        (StatusCode::OK, Json("Success")).into_response()
    } else {
        error!("Validation Failure");
        (StatusCode::UNPROCESSABLE_ENTITY, Json("Validation Failed")).into_response()
    }
}
