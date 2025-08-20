use axum::{
    Json,
    extract::State,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use rustsystem_proof::{
    Ballot, BallotMetaData, BallotValidation, Provider, RegistrationReject,
    RegistrationSuccessResponse, Sha256Provider, Sha256RegistrationInfo, Sha256ValidationInfo,
    ValidationInfo, ValidationReject,
};
use tracing::{error, info};

use crate::{
    AppState,
    api::{APIHandler, APIResult, common::common_responses::ensure_round},
    vote_auth::VoteRound,
};

use super::auth::AuthVoter;

pub struct Register;
impl APIHandler for Register {
    type State = AppState;
    type Request = (AuthVoter, State<AppState>, Json<Sha256RegistrationInfo>);
    type SuccessResponse = Json<RegistrationSuccessResponse>;
    type ErrorResponse = Json<RegistrationReject>;

    async fn handler(
        request: Self::Request,
    ) -> crate::api::APIResponse<Self::SuccessResponse, Self::ErrorResponse> {
        let (AuthVoter { uuid, muid }, State(state), Json(body)) = request;
        info!("Got register request");

        let mut meetings = state.meetings.lock().await;
        let meeting = if let Some(meeting_ok) = meetings.get_mut(&muid) {
            meeting_ok
        } else {
            return Err((
                StatusCode::NOT_FOUND,
                Json(RegistrationReject::MUIDNotFound),
            ));
        };

        let vote_auth = meeting.get_auth();

        let round = match ensure_round(vote_auth, RegistrationReject::VoteInactive) {
            Ok(_r) => _r,
            Err(err) => return Err(err),
        };

        if round.is_registered(uuid) {
            return Err((
                StatusCode::CONFLICT,
                Json(RegistrationReject::AlreadyRegistered),
            ));
        }

        if let Ok(signature) = Sha256Provider::sign_token(
            body.commitment,
            round.header().clone(),
            round.keys().clone(),
        ) {
            round.register_user(uuid);
            Ok((
                StatusCode::CREATED,
                Json(RegistrationSuccessResponse::new(
                    signature,
                    round.metadata(),
                )),
            ))
        } else {
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(RegistrationReject::SignatureFailure),
            ))
        }
    }
}

pub struct Submit;
impl APIHandler for Submit {
    type State = AppState;
    type Request = (AuthVoter, State<AppState>, Json<Ballot>);
    type SuccessResponse = ();
    type ErrorResponse = Json<ValidationReject>;
    async fn handler(
        request: Self::Request,
    ) -> crate::api::APIResponse<Self::SuccessResponse, Self::ErrorResponse> {
        let (AuthVoter { uuid, muid }, State(state), Json(body)) = request;
        let metadata = body.get_metadata();
        let choice = body.get_choice();
        let validation = body.get_validation();

        let mut meetings = state.meetings.lock().await;
        let meeting = if let Some(meeting_ok) = meetings.get_mut(&muid) {
            meeting_ok
        } else {
            return Err((StatusCode::NOT_FOUND, Json(ValidationReject::MUIDNotFound)));
        };

        let vote_auth = meeting.get_auth();

        let round = match ensure_round(vote_auth, ValidationReject::VotingInactive) {
            Ok(_r) => _r,
            Err(e) => return Err(e),
        };

        if round.is_used(validation.get_signature()) {
            return Err((
                StatusCode::CONFLICT,
                Json(ValidationReject::SignatureExpired),
            ));
        }

        validate_metadata(*metadata, &round)?;

        validate_signature(validation, round)?;

        // Only with valid metadata and a valid signature (unused!) will the vote be counted
        round.add_vote(choice.clone());

        Ok((StatusCode::OK, ()))
    }
}

fn validate_metadata(
    received: BallotMetaData,
    round: &VoteRound,
) -> APIResult<(), Json<ValidationReject>> {
    if received == round.metadata() {
        Ok(())
    } else {
        Err((
            StatusCode::CONFLICT,
            Json(ValidationReject::InvalidMetaData),
        ))
    }
}

fn validate_signature(
    validation: &BallotValidation,
    round: &mut VoteRound,
) -> APIResult<(), Json<ValidationReject>> {
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
            Json(ValidationReject::SignatureInvalid),
        ))
    }
}
