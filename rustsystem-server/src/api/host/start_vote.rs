use crate::proof::BallotMetaData;
use async_trait::async_trait;
use axum::{
    Json,
    extract::State,
    http::StatusCode,
};
use rand::seq::SliceRandom;
use serde::{Deserialize, Serialize};
use std::sync::atomic::Ordering;
use tracing::info;

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

        let candidates = metadata.get_candidates();
        let num_candidates = candidates.len();

        let meeting = state.get_meeting(auth.muuid).await?;

        // Hold the vote_auth write lock for the whole check-and-start sequence to
        // prevent a concurrent start-vote from racing past the is_inactive check.
        // The state guard is acquired BEFORE calling trustauth so that a rejected
        // duplicate never causes trustauth to replace the active round's keypair.
        let mut vote_auth = meeting.vote_auth.write().await;
        if !vote_auth.is_inactive() {
            return Err(APIError::from_error_code(APIErrorCode::InvalidState));
        }

        // Ask trustauth to generate a BLS keypair for this round; it owns the private key.
        // Called only after confirming the server is in Idle state, so the round is
        // created in trustauth if and only if the server will also transition to Voting.
        let public_key = state
            .start_round_on_trustauth(auth.muuid, &body.name)
            .await?;

        // Remove unclaimed voters and mark the meeting as locked.
        // Acquiring voters.write() while holding vote_auth.write() is safe: no other
        // operation holds voters.write() and then waits for vote_auth.write().
        let voters_before = meeting.voters.read().await.len();
        meeting.voters.write().await.retain(|_, v| v.logged_in);
        let voters_after = meeting.voters.read().await.len();
        meeting.locked.store(true, Ordering::Relaxed);

        vote_auth.start_round(metadata, body.name.clone(), public_key);

        info!(
            muuid = %auth.muuid,
            round = %body.name,
            num_candidates = num_candidates,
            shuffled = body.shuffle,
            eligible_voters = voters_after,
            unclaimed_removed = voters_before - voters_after,
            "Vote round started"
        );

        Ok(())
    }
}
