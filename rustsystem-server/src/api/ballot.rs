//! `POST /api/ballot` — the anonymous end of the protocol (`docs/PROTOCOL.md` §5.3).
//!
//! This handler takes no session extractor, so it has no way to learn who sent a ballot even
//! if a browser sends a cookie anyway. It also logs nothing: a log line per ballot would carry
//! a timestamp that could be matched against trustauth's records.

use axum::{extract::State, http::StatusCode};
use serde::Deserialize;

use rustsystem_core::{
    ApiResult, ErrorCode,
    extract::Json,
    internal::MeetingId,
    secret::b64_decode,
};

use crate::app::AppState;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Ballot {
    meeting: MeetingId,
    /// `randomizer ‖ msg`, base64url.
    prepared: String,
    /// base64url.
    sig: String,
}

pub async fn submit(State(app): State<AppState>, Json(ballot): Json<Ballot>) -> ApiResult<StatusCode> {
    let prepared = b64_decode(&ballot.prepared).ok_or(ErrorCode::MalformedBallot)?;
    let sig = b64_decode(&ballot.sig).ok_or(ErrorCode::MalformedBallot)?;
    let meeting = app.meeting(ballot.meeting).ok_or(ErrorCode::MeetingNotFound)?;
    meeting.lock().await.accept_ballot(&prepared, &sig)?;
    meeting.changed();
    Ok(StatusCode::NO_CONTENT)
}
