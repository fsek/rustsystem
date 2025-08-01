use axum::{
    Json,
    extract::State,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use rustsystem_proof::{
    Provider, RegistrationRejectReason, RegistrationResponse, Sha256Provider,
    Sha256RegistrationInfo, Sha256ValidationInfo, ValidationInfo, ValidationRejectReason,
    ValidationResponse,
};
use serde::Deserialize;
use tracing::{error, info};
use zkryptium::schemes::{algorithms::BbsBls12381Sha256, generics::BlindSignature};

use crate::AppState;

use super::auth::AuthVoter;

pub async fn register(
    AuthVoter { uuid, muid }: AuthVoter,
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
        vote_auth.header().clone(),
        vote_auth.keys().clone(),
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
pub async fn validate_vote(
    AuthVoter { uuid, muid }: AuthVoter,
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
        vote_auth.header().clone(),
        info.token,
        vote_auth.keys().public_key().clone(),
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
