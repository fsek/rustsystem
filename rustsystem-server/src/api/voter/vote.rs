use axum::{
    Json,
    extract::State,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use rustsystem_proof::{
    Ballot, BallotMetaData, BallotValidation, Provider, RegistrationRejectReason,
    RegistrationResponse, Sha256Provider, Sha256RegistrationInfo, Sha256ValidationInfo,
    ValidationInfo, ValidationRejectReason, ValidationResponse,
};
use serde::Deserialize;
use tracing::{error, info};
use zkryptium::schemes::{algorithms::BbsBls12381Sha256, generics::BlindSignature};

use crate::{
    AppState, Meeting,
    api::common::common_responses::ensure_round,
    vote_auth::{Header, VoteAuthority, VoteRound},
};

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

    let round = match ensure_round(vote_auth) {
        Ok(_r) => _r,
        Err(e) => return e.into_response(),
    };

    if round.is_registered(uuid) {
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
        round.header().clone(),
        round.keys().clone(),
    ) {
        round.register_user(uuid);
        (
            StatusCode::CREATED,
            Json(RegistrationResponse::Accepted(signature, round.metadata())),
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

pub async fn validate_vote(
    AuthVoter { uuid, muid }: AuthVoter,
    State(state): State<AppState>,
    Json(body): Json<Ballot>,
) -> Response {
    let metadata = body.get_metadata();
    let choice = body.get_choice();
    let validation = body.get_validation();

    let mut meetings = state.meetings.lock().await;
    let meeting = if let Some(meeting_ok) = meetings.get_mut(&muid) {
        meeting_ok
    } else {
        return (StatusCode::NOT_FOUND, Json("Meeting could not be found")).into_response();
    };

    let vote_auth = meeting.get_auth();

    let round = match ensure_round(vote_auth) {
        Ok(_r) => _r,
        Err(e) => return e.into_response(),
    };

    if round.is_used(validation.get_signature()) {
        return (
            StatusCode::CONFLICT,
            Json(ValidationResponse::Rejected(
                ValidationRejectReason::SignatureExpired,
            )),
        )
            .into_response();
    }

    if let Err(e) = validate_metadata(*metadata, &round) {
        return e.into_response();
    }

    if let Err(e) = validate_signature(validation, round) {
        return e.into_response();
    }

    (StatusCode::OK, Json(ValidationResponse::Accepted)).into_response()
}

fn validate_metadata(
    received: BallotMetaData,
    round: &VoteRound,
) -> Result<(), (StatusCode, Json<ValidationResponse>)> {
    if received == round.metadata() {
        Ok(())
    } else {
        Err((
            StatusCode::CONFLICT,
            Json(ValidationResponse::Rejected(
                ValidationRejectReason::InvalidMetaData,
            )),
        ))
    }
}

fn validate_signature(
    validation: &BallotValidation,
    round: &mut VoteRound,
) -> Result<(), (StatusCode, Json<ValidationResponse>)> {
    let info = Sha256ValidationInfo::from(validation.clone());

    if let Ok(_) = Sha256Provider::validate_token(
        info.get_proof(),
        round.header().clone(),
        info.token,
        round.keys().public_key().clone(),
        info.signature.clone(),
    ) {
        info!("Validation Successful");
        round.set_signature_expired(&info.signature);
        Ok(())
    } else {
        error!("Validation Failure");
        Err((
            StatusCode::UNPROCESSABLE_ENTITY,
            Json(ValidationResponse::Rejected(
                ValidationRejectReason::SignatureInvalid,
            )),
        ))
    }
}
