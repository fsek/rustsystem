//! The internal API between the server and trustauth (`docs/PROTOCOL.md` §4–5).
//!
//! Traffic only ever goes server → trustauth, over mTLS. Both crates use these types, so the
//! two sides cannot drift apart.

use serde::{Deserialize, Serialize};
use uuid::Uuid;

pub type MeetingId = Uuid;
pub type VoterId = Uuid;
pub type RoundId = Uuid;

/// Trustauth login tickets are valid for this long.
pub const TICKET_TTL_SECS: u64 = 60;

pub mod paths {
    pub const TICKETS: &str = "/internal/tickets";
    pub const ROUNDS: &str = "/internal/rounds";
    pub const MEETINGS: &str = "/internal/meetings";
    pub const VOTERS: &str = "/internal/voters";
}

/// `POST /internal/tickets` — the server vouches that whoever presents `ticket` within
/// [`TICKET_TTL_SECS`] is `voter` in `meeting`.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct IssueTicket {
    pub ticket: String,
    pub meeting: MeetingId,
    pub voter: VoterId,
}

/// `POST /internal/rounds` — open a round. Replaces any previous round for the meeting.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct StartRound {
    pub meeting: MeetingId,
    pub round: RoundId,
    pub eligible: Vec<VoterId>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct StartRoundResponse {
    /// SPKI DER, base64url.
    pub public_key: String,
}

/// `GET /internal/rounds/{meeting}`.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct RoundStatus {
    pub round: Option<RoundId>,
    pub signed: usize,
}

// `DELETE /internal/rounds/{meeting}`: drop the round (closed or cancelled).
// `DELETE /internal/meetings/{meeting}`: drop the round and every session for the meeting.
// `DELETE /internal/voters/{meeting}/{voter}`: drop one voter's sessions (removed or reset).
