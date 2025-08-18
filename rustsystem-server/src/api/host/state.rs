use std::io;

use axum::{
    Json,
    extract::State,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use rustsystem_proof::{BallotMetaData, ProtocolVersion, VoteMethod};
use serde::{Deserialize, Serialize};
use tracing::info;

use crate::{AppState, api::common::common_responses::ensure_round, vote_auth::Tally};

use super::auth::AuthHost;

#[derive(Deserialize)]
pub struct StartVoteRequest {
    name: String,
    metadata: BallotMetaData,
}

pub async fn start_vote(
    AuthHost { uuid, muid }: AuthHost,
    State(state): State<AppState>,
    Json(body): Json<StartVoteRequest>,
) -> Response {
    if let Some(meeting) = state.meetings.lock().await.get_mut(&muid) {
        info!("Starting vote: {}", body.name);
        meeting.get_auth().start_round(body.metadata, body.name);

        return StatusCode::OK.into_response();
    } else {
        return StatusCode::NOT_FOUND.into_response();
    }
}

pub async fn tally(AuthHost { uuid, muid }: AuthHost, State(state): State<AppState>) -> Response {
    if let Some(meeting) = state.meetings.lock().await.get_mut(&muid) {
        let vote_auth = meeting.get_auth();

        match vote_auth.finalize_round() {
            Ok(res) => (StatusCode::OK, Json(res)).into_response(),
            Err(e) => e.into_response(),
        }
    } else {
        return StatusCode::NOT_FOUND.into_response();
    }
}
