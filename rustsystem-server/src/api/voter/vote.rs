use api_derive::APIEndpointError;
use axum::{Json, extract::State, http::StatusCode};
use rustsystem_proof::{
    Ballot, BallotMetaData, BallotValidation, Provider, RegistrationSuccessResponse,
    Sha256Provider, Sha256RegistrationInfo, Sha256ValidationInfo, ValidationInfo,
};
use tracing::{error, info};

use api_core::{APIErrorCode, APIHandler, APIResult};

use crate::{AppState, api::common::common_responses::ensure_round, vote_auth::VoteRound};

use super::auth::AuthVoter;

#[derive(APIEndpointError)]
#[api(endpoint(method = "POST", path = "/api/voter/register"))]
pub enum RegisterError {
    #[api(code = APIErrorCode::SignatureFailure, status = 500)]
    SignatureFailure,
    #[api(code = APIErrorCode::AlreadyRegistered, status = 409)]
    AlreadyRegistered,

    #[api(code = APIErrorCode::MUuidNotFound, status = 404)]
    MUIDNotFound,
    #[api(code = APIErrorCode::VotingInactive, status = 410)]
    VoteInactive,
}

pub struct Register;
impl APIHandler for Register {
    type State = AppState;
    type Request = (AuthVoter, State<AppState>, Json<Sha256RegistrationInfo>);

    const SUCCESS_CODE: StatusCode = StatusCode::CREATED;
    type SuccessResponse = Json<RegistrationSuccessResponse>;
    type ErrorResponse = RegisterError;

    async fn route(
        request: Self::Request,
    ) -> APIResult<Self::SuccessResponse, Self::ErrorResponse> {
        let (AuthVoter { uuuid, muuid }, State(state), Json(body)) = request;
        info!("Got register request");

        let mut meetings = state.meetings.lock().await;
        let meeting = if let Some(meeting_ok) = meetings.get_mut(&muuid) {
            meeting_ok
        } else {
            return Err(RegisterError::MUIDNotFound);
        };

        let vote_auth = meeting.get_auth();

        let round = ensure_round(vote_auth, RegisterError::VoteInactive)?;

        if round.is_registered(uuuid) {
            return Err(RegisterError::AlreadyRegistered);
        }

        if let Ok(signature) = Sha256Provider::sign_token(
            body.commitment,
            round.header().clone(),
            round.keys().clone(),
        ) {
            round.register_user(uuuid);
            Ok(Json(RegistrationSuccessResponse::new(
                signature,
                round.metadata(),
            )))
        } else {
            Err(RegisterError::SignatureFailure)
        }
    }
}

#[derive(APIEndpointError)]
#[api(endpoint(method = "POST", path = "/api/voter/submit"))]
pub enum SubmitError {
    #[api(code = APIErrorCode::InvalidMetaData, status = 409)]
    InvalidMetaData,
    #[api(code = APIErrorCode::MUuidNotFound, status = 404)]
    MUIDNotFound,
    #[api(code = APIErrorCode::VotingInactive, status = 410)]
    VotingInactive,

    #[api(code = APIErrorCode::SignatureInvalid, status = 422)]
    SignatureInvalid,
    #[api(code = APIErrorCode::SignatureExpired, status = 409)]
    SignatureExpired,
}

pub struct Submit;
impl APIHandler for Submit {
    type State = AppState;
    type Request = (AuthVoter, State<AppState>, Json<Ballot>);

    const SUCCESS_CODE: StatusCode = StatusCode::OK;
    type SuccessResponse = ();
    type ErrorResponse = SubmitError;
    async fn route(
        request: Self::Request,
    ) -> APIResult<Self::SuccessResponse, Self::ErrorResponse> {
        let (AuthVoter { uuuid, muuid }, State(state), Json(body)) = request;
        let metadata = body.get_metadata();
        let choice = body.get_choice();
        let validation = body.get_validation();

        let mut meetings = state.meetings.lock().await;
        let meeting = if let Some(meeting_ok) = meetings.get_mut(&muuid) {
            meeting_ok
        } else {
            return Err(SubmitError::MUIDNotFound);
        };

        let vote_auth = meeting.get_auth();

        let round = ensure_round(vote_auth, SubmitError::VotingInactive)?;

        if round.is_used(validation.get_signature()) {
            return Err(SubmitError::SignatureExpired);
        }

        validate_metadata(*metadata, round)?;

        validate_signature(validation, round)?;

        // Only with valid metadata and a valid signature (unused!) will the vote be counted
        round.add_vote(choice.clone());

        Ok(())
    }
}

fn validate_metadata(received: BallotMetaData, round: &VoteRound) -> APIResult<(), SubmitError> {
    if received == round.metadata() {
        Ok(())
    } else {
        Err(SubmitError::InvalidMetaData)
    }
}

fn validate_signature(
    validation: &BallotValidation,
    round: &mut VoteRound,
) -> APIResult<(), SubmitError> {
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
        Err(SubmitError::SignatureInvalid)
    }
}
