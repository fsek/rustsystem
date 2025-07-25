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
    ValidationRejectReason, ValidationResponse,
};
use serde::{Deserialize, Serialize};
use tokio::sync::watch::{self, Receiver, Sender};
use tracing::{error, info};
use zkryptium::{
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

pub struct VoteAuthority {
    keys: AuthenticationKeys,
    header: Header,
    registered_voters: HashSet<UUID>,
    expired_signatures: HashSet<[u8; 80]>,
    state_tx: Sender<bool>,
}
impl VoteAuthority {
    /// For new meeting
    pub fn new(header: String) -> Self {
        let keys = AuthenticationKeys(Sha256Provider::generate_authentication_keys());
        let header = Header(header.as_bytes().to_vec());
        let registered_voters = HashSet::new();
        let expired_signatures = HashSet::new();

        Self {
            keys,
            header,
            registered_voters,
            expired_signatures,
            state_tx: Sender::new(false),
        }
    }

    pub fn is_active(&self) -> bool {
        *self.state_tx.borrow()
    }
    pub fn set_active_state(&mut self, new_state: bool) {
        self.state_tx.send(new_state);
    }
    pub fn new_watcher(&self) -> Receiver<bool> {
        self.state_tx.subscribe()
    }

    /// Resets VoteAuth for new voting round. Old ballots are no longer valid since the
    /// keys have changed.
    /// Voters can now re-register.
    pub fn reset(&mut self) {
        self.keys.0 = Sha256Provider::generate_authentication_keys();
        self.registered_voters.clear();
        self.expired_signatures.clear();
    }

    /// Checks if a user has already registered for voting
    pub fn is_registered(&self, uuid: UUID) -> bool {
        self.registered_voters.contains(&uuid)
    }

    pub fn register_user(&mut self, uuid: UUID) {
        self.registered_voters.insert(uuid);
    }

    pub fn is_used(&self, signature: &BlindSignature<BbsBls12381Sha256>) -> bool {
        self.expired_signatures.contains(&signature.to_bytes())
    }

    pub fn set_signature_expired(&mut self, signature: &BlindSignature<BbsBls12381Sha256>) {
        self.expired_signatures.insert(signature.to_bytes());
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
        vote_auth.register_user(uuid);
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

    if vote_auth.is_used(&body.signature) {
        return (
            StatusCode::CONFLICT,
            Json(ValidationResponse::Rejected(
                ValidationRejectReason::SignatureExpired,
            )),
        )
            .into_response();
    }

    let info = Sha256ValidationInfo::new(body.proof, body.token, body.signature);

    if let Ok(_) = Sha256Provider::validate_token(
        info.get_proof(),
        vote_auth.header.0.clone(),
        info.token,
        vote_auth.keys.0.public_key().clone(),
        info.signature.clone(),
    ) {
        info!("Validation Successful");
        vote_auth.set_signature_expired(&info.signature);
        (StatusCode::OK, Json(ValidationResponse::Accepted)).into_response()
    } else {
        error!("Validation Failure");
        (
            StatusCode::UNPROCESSABLE_ENTITY,
            Json(ValidationResponse::Rejected(
                ValidationRejectReason::SignatureInvalid,
            )),
        )
            .into_response()
    }
}
