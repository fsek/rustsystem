use axum::{Json, http::StatusCode};
use rustsystem_proof::{ValidationRejectReason, ValidationResponse};

use crate::vote_auth::{VoteAuthority, VoteRound};

pub fn ensure_round(
    vote_auth: &mut VoteAuthority,
) -> Result<&mut VoteRound, (StatusCode, Json<ValidationResponse>)> {
    if let Some(round) = vote_auth.round() {
        Ok(round)
    } else {
        Err((
            StatusCode::GONE,
            Json(ValidationResponse::Rejected(
                ValidationRejectReason::VotingInactive,
            )),
        ))
    }
}
