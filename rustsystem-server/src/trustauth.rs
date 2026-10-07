//! The server's client for trustauth's internal API (mTLS in production).
//!
//! Calls that a request depends on (tickets, opening a round) return
//! [`ErrorCode::TrustauthUnavailable`] on failure. Cleanup calls are best effort: if trustauth
//! misses one, its own pruning catches up.

use reqwest::Client;
use serde::de::DeserializeOwned;
use tracing::warn;

use rustsystem_core::{
    ApiError, ApiResult, ErrorCode,
    internal::{
        IssueTicket, MeetingId, RoundId, RoundStatus, StartRound, StartRoundResponse, VoterId,
        paths,
    },
    secret::new_token,
};

#[derive(Clone)]
pub struct Trustauth {
    http: Client,
    base: String,
}

impl Trustauth {
    /// `base` is trustauth's internal URL, e.g. `https://rustsystem-trustauth:2444`.
    pub fn new(http: Client, base: impl Into<String>) -> Self {
        Self {
            http,
            base: base.into().trim_end_matches('/').to_owned(),
        }
    }

    fn unavailable(e: impl std::fmt::Display) -> ApiError {
        warn!("trustauth call failed: {e}");
        ApiError::new(ErrorCode::TrustauthUnavailable)
    }

    async fn json<T: DeserializeOwned>(req: reqwest::RequestBuilder) -> ApiResult<T> {
        req.send()
            .await
            .and_then(|r| r.error_for_status())
            .map_err(Self::unavailable)?
            .json()
            .await
            .map_err(Self::unavailable)
    }

    async fn send(req: reqwest::RequestBuilder) -> ApiResult<()> {
        req.send()
            .await
            .and_then(|r| r.error_for_status())
            .map(|_| ())
            .map_err(Self::unavailable)
    }

    /// Registers a fresh login ticket for `voter` and returns it for the browser.
    pub async fn issue_ticket(&self, meeting: MeetingId, voter: VoterId) -> ApiResult<String> {
        let ticket = new_token();
        let body = IssueTicket { ticket: ticket.clone(), meeting, voter };
        Self::send(self.http.post(format!("{}{}", self.base, paths::TICKETS)).json(&body)).await?;
        Ok(ticket)
    }

    /// Opens a round on trustauth and returns its public key (SPKI DER, base64url).
    pub async fn start_round(&self, meeting: MeetingId, round: RoundId, eligible: Vec<VoterId>) -> ApiResult<String> {
        let body = StartRound { meeting, round, eligible };
        let resp: StartRoundResponse =
            Self::json(self.http.post(format!("{}{}", self.base, paths::ROUNDS)).json(&body)).await?;
        Ok(resp.public_key)
    }

    pub async fn round_status(&self, meeting: MeetingId) -> ApiResult<RoundStatus> {
        Self::json(self.http.get(format!("{}{}/{meeting}", self.base, paths::ROUNDS))).await
    }

    pub async fn drop_round(&self, meeting: MeetingId) {
        let _ = Self::send(self.http.delete(format!("{}{}/{meeting}", self.base, paths::ROUNDS))).await;
    }

    pub async fn drop_meeting(&self, meeting: MeetingId) {
        let _ = Self::send(self.http.delete(format!("{}{}/{meeting}", self.base, paths::MEETINGS))).await;
    }

    pub async fn drop_voter(&self, meeting: MeetingId, voter: VoterId) {
        let _ = Self::send(self.http.delete(format!("{}{}/{meeting}/{voter}", self.base, paths::VOTERS))).await;
    }
}
