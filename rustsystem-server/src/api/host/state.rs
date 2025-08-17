use axum::{
    Json,
    extract::State,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use rustsystem_proof::{BallotMetaData, ProtocolVersion, VoteMethod};
use serde::Deserialize;
use tracing::info;

use crate::{AppState, api::common::common_responses::ensure_round};

use super::auth::AuthHost;

#[derive(Deserialize)]
pub struct StartVoteRequest {
    name: String,
    method: VoteMethod,
    protocol_version: ProtocolVersion,
}

pub async fn start_vote(
    AuthHost { uuid, muid }: AuthHost,
    State(state): State<AppState>,
    Json(body): Json<StartVoteRequest>,
) -> Response {
    if let Some(meeting) = state.meetings.lock().await.get_mut(&muid) {
        info!("Starting vote: {}", body.name);
        meeting.get_auth().start_round(
            BallotMetaData::new(body.method, body.protocol_version),
            body.name,
        );

        return StatusCode::OK.into_response();
    } else {
        return StatusCode::NOT_FOUND.into_response();
    }
}

pub async fn stop_vote(
    AuthHost { uuid, muid }: AuthHost,
    State(state): State<AppState>,
) -> Response {
    if let Some(meeting) = state.meetings.lock().await.get_mut(&muid) {
        let vote_auth = meeting.get_auth();

        vote_auth.finalize_round();
        return StatusCode::OK.into_response();
    } else {
        return StatusCode::NOT_FOUND.into_response();
    }
}
