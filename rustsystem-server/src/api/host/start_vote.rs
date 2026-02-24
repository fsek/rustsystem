use crate::proof::BallotMetaData;
use async_trait::async_trait;
use axum::{
    Json,
    extract::State,
    http::StatusCode,
};
use rand::seq::SliceRandom;
use serde::{Deserialize, Serialize};

use rustsystem_core::{APIError, APIErrorCode, APIHandler, Method};

use crate::AppState;

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

        let meetings = state.meetings()?;

        if let Some(meeting) = meetings.lock().await.get_mut(&auth.muuid) {
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
