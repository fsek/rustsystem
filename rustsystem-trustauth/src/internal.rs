//! Trustauth's internal API, called only by the server over mTLS (`docs/PROTOCOL.md` §4–5).
//! Request and response types live in `rustsystem_core::internal` so both sides share them.

use std::time::Instant;

use axum::{extract::State, http::StatusCode};
use tracing::info;

use rustsystem_core::{
    ApiError, ApiResult,
    blind::RoundKey,
    extract::{Json, Path},
    internal::{IssueTicket, MeetingId, RoundStatus, StartRound, StartRoundResponse, VoterId},
    secret::b64_encode,
};

use crate::{AppState, state::Identity};

pub async fn issue_ticket(State(app): State<AppState>, Json(req): Json<IssueTicket>) -> StatusCode {
    let who = Identity {
        meeting: req.meeting,
        voter: req.voter,
    };
    app.state.issue_ticket(&req.ticket, who, Instant::now());
    StatusCode::NO_CONTENT
}

pub async fn start_round(
    State(app): State<AppState>,
    Json(req): Json<StartRound>,
) -> ApiResult<Json<StartRoundResponse>> {
    let key = tokio::task::spawn_blocking(RoundKey::generate)
        .await
        .map_err(ApiError::internal)?
        .map_err(|e| ApiError::internal(format!("round key generation: {e:?}")))?;
    let public_key = key
        .public_key()
        .to_der()
        .map_err(|e| ApiError::internal(format!("public key encoding: {e:?}")))?;

    let eligible = req.eligible.len();
    app.state
        .start_round(req.meeting, req.round, req.eligible, key, Instant::now());
    info!(meeting = %req.meeting, round = %req.round, eligible, "Round opened, key generated");

    Ok(Json(StartRoundResponse {
        public_key: b64_encode(public_key),
    }))
}

pub async fn round_status(
    State(app): State<AppState>,
    Path(meeting): Path<MeetingId>,
) -> Json<RoundStatus> {
    Json(app.state.round_status(meeting))
}

pub async fn drop_round(State(app): State<AppState>, Path(meeting): Path<MeetingId>) -> StatusCode {
    app.state.drop_round(meeting);
    info!(meeting = %meeting, "Round dropped");
    StatusCode::NO_CONTENT
}

pub async fn drop_meeting(
    State(app): State<AppState>,
    Path(meeting): Path<MeetingId>,
) -> StatusCode {
    app.state.drop_meeting(meeting);
    info!(meeting = %meeting, "Meeting dropped");
    StatusCode::NO_CONTENT
}

pub async fn drop_voter(
    State(app): State<AppState>,
    Path((meeting, voter)): Path<(MeetingId, VoterId)>,
) -> StatusCode {
    app.state.drop_voter(Identity { meeting, voter });
    StatusCode::NO_CONTENT
}
