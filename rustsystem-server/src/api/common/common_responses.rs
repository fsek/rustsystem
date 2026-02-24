use rustsystem_core::{APIError, APIErrorCode};

use crate::vote_auth::{VoteAuthority, VoteRound};

pub fn ensure_round(vote_auth: &mut VoteAuthority) -> Result<&mut VoteRound, APIError> {
    if let Some(round) = vote_auth.round() {
        Ok(round)
    } else {
        Err(APIError::from_error_code(APIErrorCode::VotingInactive))
    }
}
