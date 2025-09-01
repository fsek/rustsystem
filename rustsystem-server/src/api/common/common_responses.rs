use api_core::APIEndpointError;

use crate::vote_auth::{VoteAuthority, VoteRound};

pub fn ensure_round<E: APIEndpointError>(
    vote_auth: &mut VoteAuthority,
    err: E,
) -> Result<&mut VoteRound, E> {
    if let Some(round) = vote_auth.round() {
        Ok(round)
    } else {
        Err(err)
    }
}
