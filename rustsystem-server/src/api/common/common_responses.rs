use axum::{Json, http::StatusCode};
use serde::Serialize;

use crate::vote_auth::{VoteAuthority, VoteRound};

pub fn ensure_round<T: Serialize>(
    vote_auth: &mut VoteAuthority,
    err: T,
) -> Result<&mut VoteRound, (StatusCode, Json<T>)> {
    if let Some(round) = vote_auth.round() {
        Ok(round)
    } else {
        Err((StatusCode::GONE, Json(err)))
    }
}
