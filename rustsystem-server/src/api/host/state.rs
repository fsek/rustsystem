use crate::proof::BallotMetaData;
use async_trait::async_trait;
use axum::{
    Json,
    extract::{FromRequest, State},
    http::StatusCode,
};
use rand::seq::SliceRandom;
use serde::{Deserialize, Serialize};

use api_core::{APIError, APIErrorCode, APIHandler, Method};

use crate::{AppState, tally_encrypt::save_encrypted_tally, vote_auth};

use super::auth::AuthHost;

#[derive(Deserialize, Serialize)]
pub struct StartVoteRequest {
    pub name: String,
    pub shuffle: bool,
    pub metadata: BallotMetaData,
}

pub struct StartVote;
#[async_trait]
impl APIHandler for StartVote {
    type State = AppState;
    type Request = (AuthHost, State<AppState>, Json<StartVoteRequest>);
    type SuccessResponse = ();

    const METHOD: Method = Method::Post;
    const PATH: &'static str = "/start-vote";
    const SUCCESS_CODE: StatusCode = StatusCode::OK;

    async fn route(request: Self::Request) -> Result<Self::SuccessResponse, APIError> {
        let (auth, State(state), Json(body)) = request;

        if !body.metadata.check_valid() {
            return Err(APIError::from_error_code(APIErrorCode::InvalidMetaData));
        }

        // Shuffle candidates before sending to trustauth so both sides agree on the order.
        let mut metadata = body.metadata;
        if body.shuffle {
            let mut candidates = metadata.get_candidates();
            candidates.shuffle(&mut rand::rng());
            metadata.set_candidates(candidates);
        }

        // Ask trustauth to generate a BLS keypair for this round; it owns the private key.
        let public_key = state
            .start_round_on_trustauth(auth.muuid, &body.name)
            .await?;

        let meetings_guard = {
            let guard = state.read()?;
            guard.clone().meetings
        };

        if let Some(meeting) = meetings_guard.lock().await.get_mut(&auth.muuid) {
            if meeting.get_auth().is_inactive() {
                meeting.lock();
                meeting
                    .get_auth()
                    .start_round(metadata, body.name, public_key);
            } else {
                return Err(APIError::from_error_code(APIErrorCode::InvalidState));
            }

            Ok(())
        } else {
            Err(APIError::from_error_code(APIErrorCode::MUuidNotFound))
        }
    }
}

#[derive(FromRequest)]
pub struct TallyRequest {
    auth: AuthHost,
    state: State<AppState>,
}

pub struct Tally;
#[async_trait]
impl APIHandler for Tally {
    type State = AppState;
    type Request = TallyRequest;
    type SuccessResponse = Json<vote_auth::Tally>;

    const METHOD: Method = Method::Post;
    const PATH: &'static str = "/tally";
    const SUCCESS_CODE: StatusCode = StatusCode::OK;

    async fn route(request: Self::Request) -> Result<Self::SuccessResponse, APIError> {
        let TallyRequest {
            auth,
            state: State(state),
        } = request;

        let meetings_guard = {
            let guard = state.read()?;
            guard.clone().meetings
        };

        if let Some(meeting) = meetings_guard.lock().await.get_mut(&auth.muuid) {
            let vote_auth = meeting.get_auth();

            let tally_result = vote_auth.finalize_round()?;

            // Unlock the meeting during tally phase to allow invitations between voting sessions.
            // This enables hosts to invite new participants while results are being displayed,
            // before starting the next vote round. The meeting will remain unlocked until
            // a new vote starts (which locks it again).
            meeting.unlock();

            Ok(Json(tally_result))
        } else {
            Err(APIError::from_error_code(APIErrorCode::MUuidNotFound))
        }
    }
}

#[derive(FromRequest)]
pub struct GetTallyRequest {
    auth: AuthHost,
    state: State<AppState>,
}

pub struct GetTally;
#[async_trait]
impl APIHandler for GetTally {
    type State = AppState;
    type Request = GetTallyRequest;
    type SuccessResponse = Json<vote_auth::Tally>;

    const METHOD: Method = Method::Get;
    const PATH: &'static str = "/get-tally";
    const SUCCESS_CODE: StatusCode = StatusCode::OK;

    async fn route(request: Self::Request) -> Result<Self::SuccessResponse, APIError> {
        let GetTallyRequest {
            auth,
            state: State(state),
        } = request;

        let meetings_guard = {
            let guard = state.read()?;
            guard.clone().meetings
        };

        if let Some(meeting) = meetings_guard.lock().await.get(&auth.muuid) {
            if let Some(tally) = meeting.vote_auth.get_last_tally() {
                if let Err(e) = save_encrypted_tally(
                    &auth.muuid,
                    tally,
                    meeting
                        .voters
                        .iter()
                        .map(|(_k, v)| v.name.clone())
                        .collect(),
                ) {
                    tracing::error!(
                        "Failed to save encrypted tally for meeting {}: {e}",
                        auth.muuid
                    );
                }
                Ok(Json(tally.clone()))
            } else {
                Err(APIError::from_error_code(APIErrorCode::InvalidState))
            }
        } else {
            Err(APIError::from_error_code(APIErrorCode::MUuidNotFound))
        }
    }
}

#[derive(FromRequest)]
pub struct EndVoteRoundRequest {
    auth: AuthHost,
    state: State<AppState>,
}

pub struct EndVoteRound;
#[async_trait]
impl APIHandler for EndVoteRound {
    type State = AppState;
    type Request = EndVoteRoundRequest;
    type SuccessResponse = ();

    const METHOD: Method = Method::Delete;
    const PATH: &'static str = "/end-vote-round";
    const SUCCESS_CODE: StatusCode = StatusCode::OK;

    async fn route(request: Self::Request) -> Result<Self::SuccessResponse, APIError> {
        let EndVoteRoundRequest {
            auth,
            state: State(state),
        } = request;

        let meetings_guard = {
            let guard = state.read()?;
            guard.clone().meetings
        };
        if let Some(meeting) = meetings_guard.lock().await.get_mut(&auth.muuid) {
            meeting.get_auth().reset();
            // Upon a hard reset (i.e. cancelling the voting round), we unlock
            meeting.unlock();
            Ok(())
        } else {
            Err(APIError::from_error_code(APIErrorCode::MUuidNotFound))
        }
    }
}
