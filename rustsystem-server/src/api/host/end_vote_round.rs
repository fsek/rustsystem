use async_trait::async_trait;
use axum::{
    extract::{FromRequest, State},
    http::StatusCode,
};
use tracing::info;

use rustsystem_core::{APIError, APIHandler, Method};

use crate::AppState;

use super::auth::AuthHost;

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

        let meeting = state.get_meeting(auth.muuid).await?;

        let round_name = meeting
            .vote_auth
            .read()
            .await
            .get_current_vote_name()
            .cloned();

        meeting.vote_auth.write().await.reset();
        // Upon a hard reset (i.e. cancelling the voting round), we unlock
        meeting.unlock();

        // Remove the round's BLS keypair and registered-voter set from trustauth,
        // but only if a round was actually active (round_name is None in Creation state).
        if round_name.is_some() {
            state.end_round_on_trustauth(auth.muuid).await?;
        }

        info!(
            muuid = %auth.muuid,
            round = %round_name.as_deref().unwrap_or("(none)"),
            "Vote round ended (state reset to creation)"
        );

        Ok(())
    }
}
