use crate::proof::{
    Ballot, BallotMetaData, BallotValidation, Choice, Provider, Sha256Provider,
    Sha256ValidationInfo, ValidationInfo,
};
use async_trait::async_trait;
use axum::{Json, extract::State, http::StatusCode};
use tracing::{error, info};

use rustsystem_core::{APIError, APIErrorCode, APIHandler, Method};

use crate::{
    AppState, api::common::common_responses::ensure_round, tokens::AuthUser, vote_auth::VoteRound,
};

pub struct Submit;
#[async_trait]
impl APIHandler for Submit {
    type State = AppState;
    type Request = (AuthUser, State<AppState>, Json<Ballot>);
    type SuccessResponse = ();

    const METHOD: Method = Method::Post;
    const PATH: &'static str = "/submit";
    const SUCCESS_CODE: StatusCode = StatusCode::OK;

    async fn route(request: Self::Request) -> Result<Self::SuccessResponse, APIError> {
        let (auth, State(state), Json(body)) = request;
        let metadata = body.get_metadata();
        let choice = body.get_choice();
        let validation = body.get_validation();

        let meeting = state.get_meeting(auth.muuid).await?;
        let mut vote_auth = meeting.vote_auth.write().await;
        let round = ensure_round(&mut *vote_auth)?;

        if round.is_used(validation.get_signature()) {
            return Err(APIError::from_error_code(APIErrorCode::SignatureExpired));
        }

        validate_metadata(metadata.clone(), round)?;
        validate_num_choices(choice.clone(), round)?;
        validate_signature(validation, round)?;

        // Only with valid metadata, valid length, and a valid signature (unused!) will the vote be counted
        round.add_vote(choice.to_owned());

        // Notify watchers that the vote count has been updated
        vote_auth.send_update();

        Ok(())
    }
}

fn validate_metadata(received: BallotMetaData, round: &VoteRound) -> Result<(), APIError> {
    if received == round.metadata() {
        Ok(())
    } else {
        Err(APIError::from_error_code(APIErrorCode::InvalidMetaData))
    }
}

fn validate_num_choices(choice: Option<Choice>, round: &VoteRound) -> Result<(), APIError> {
    if let Some(choices) = choice.as_ref()
        && choices.len() > round.metadata().get_max_choices()
    {
        return Err(APIError::from_error_code(APIErrorCode::InvalidVoteLength));
    }

    Ok(())
}

fn validate_signature(
    validation: &BallotValidation,
    round: &mut VoteRound,
) -> Result<(), APIError> {
    let info = Sha256ValidationInfo::from(validation.clone());

    if Sha256Provider::validate_token(
        info.get_proof(),
        round.header().clone(),
        info.token,
        round.public_key().clone(),
        info.signature.clone(),
    )
    .is_ok()
    {
        info!("Validation Successful");
        round.set_signature_expired(&info.signature);
        Ok(())
    } else {
        error!("Validation Failure");
        Err(APIError::from_error_code(APIErrorCode::SignatureInvalid))
    }
}
