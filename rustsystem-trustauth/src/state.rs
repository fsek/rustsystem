//! Everything trustauth knows, and every rule it enforces (`docs/PROTOCOL.md` §4.3, §5.2).
//!
//! Trustauth knows *who* someone is. Per meeting it holds:
//! - sessions: `SHA-256(cookie)` → (meeting, voter);
//! - pending login tickets from the server;
//! - the current round: its key, the eligible voter IDs, and the IDs already signed.
//!
//! It never stores blinded messages or signatures. All methods here are synchronous and hold
//! the lock only briefly; the one slow operation (RSA signing) happens outside it.

use std::{
    collections::{HashMap, HashSet},
    sync::{Arc, Mutex, MutexGuard},
    time::{Duration, Instant},
};

use rustsystem_core::{
    ApiResult, ErrorCode,
    blind::RoundKey,
    internal::{MeetingId, RoundId, RoundStatus, TICKET_TTL_SECS, VoterId},
    secret::{TokenHash, hash_token, new_token},
    session::SESSION_TTL,
};

/// A round left behind (e.g. the server never told us it ended) is dropped after this long.
pub const ROUND_TTL: Duration = Duration::from_secs(13 * 60 * 60);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Identity {
    pub meeting: MeetingId,
    pub voter: VoterId,
}

struct Expiring {
    who: Identity,
    expires: Instant,
}

struct Round {
    id: RoundId,
    key: Arc<RoundKey>,
    eligible: HashSet<VoterId>,
    signed: HashSet<VoterId>,
    started: Instant,
}

#[derive(Default)]
struct Inner {
    sessions: HashMap<TokenHash, Expiring>,
    tickets: HashMap<TokenHash, Expiring>,
    rounds: HashMap<MeetingId, Round>,
}

/// What a voter's page needs after a refresh: is there a round, and have I been signed in it?
#[derive(serde::Serialize, Debug, PartialEq, Eq)]
pub struct VoterStatus {
    pub round: Option<RoundId>,
    pub signed: bool,
}

#[derive(Clone, Default)]
pub struct State(Arc<Mutex<Inner>>);

impl State {
    fn lock(&self) -> MutexGuard<'_, Inner> {
        // A panic while holding the lock can't leave these maps half-updated in a way that
        // matters more than refusing all further requests would, so recover the guard.
        self.0.lock().unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    // ── Login ────────────────────────────────────────────────────────────────

    pub fn issue_ticket(&self, ticket: &str, who: Identity, now: Instant) {
        let expires = now + Duration::from_secs(TICKET_TTL_SECS);
        self.lock().tickets.insert(hash_token(ticket), Expiring { who, expires });
    }

    /// Consumes the ticket and returns a new session token for the cookie.
    pub fn redeem_ticket(&self, ticket: &str, now: Instant) -> ApiResult<(String, Identity)> {
        let mut inner = self.lock();
        let entry = inner
            .tickets
            .remove(&hash_token(ticket))
            .filter(|t| t.expires > now)
            .ok_or(ErrorCode::TicketInvalid)?;
        let token = new_token();
        inner.sessions.insert(
            hash_token(&token),
            Expiring {
                who: entry.who,
                expires: now + SESSION_TTL,
            },
        );
        Ok((token, entry.who))
    }

    pub fn session(&self, token: &str, now: Instant) -> Option<Identity> {
        let inner = self.lock();
        let s = inner.sessions.get(&hash_token(token))?;
        (s.expires > now).then_some(s.who)
    }

    pub fn logout(&self, token: &str) {
        self.lock().sessions.remove(&hash_token(token));
    }

    // ── Rounds ───────────────────────────────────────────────────────────────

    /// Opens a round, replacing any previous one for the meeting.
    pub fn start_round(
        &self,
        meeting: MeetingId,
        id: RoundId,
        eligible: Vec<VoterId>,
        key: RoundKey,
        now: Instant,
    ) {
        let round = Round {
            id,
            key: Arc::new(key),
            eligible: eligible.into_iter().collect(),
            signed: HashSet::new(),
            started: now,
        };
        self.lock().rounds.insert(meeting, round);
    }

    /// Checks that `who` may get a signature in `round` and records that they have.
    /// Returns the key to sign with; the caller signs *outside* the lock.
    pub fn claim_signature(&self, who: Identity, round: RoundId) -> ApiResult<Arc<RoundKey>> {
        let mut inner = self.lock();
        let current = inner.rounds.get_mut(&who.meeting).ok_or(ErrorCode::VotingClosed)?;
        if current.id != round {
            return Err(ErrorCode::WrongRound.into());
        }
        if !current.eligible.contains(&who.voter) {
            return Err(ErrorCode::NotEligible.into());
        }
        if !current.signed.insert(who.voter) {
            return Err(ErrorCode::AlreadySigned.into());
        }
        Ok(current.key.clone())
    }

    /// Undoes [`claim_signature`](Self::claim_signature) when signing failed, so a voter whose
    /// request was malformed can try again.
    pub fn release_signature(&self, who: Identity, round: RoundId) {
        if let Some(current) = self.lock().rounds.get_mut(&who.meeting)
            && current.id == round
        {
            current.signed.remove(&who.voter);
        }
    }

    pub fn voter_status(&self, who: Identity) -> VoterStatus {
        match self.lock().rounds.get(&who.meeting) {
            Some(r) => VoterStatus {
                round: Some(r.id),
                signed: r.signed.contains(&who.voter),
            },
            None => VoterStatus {
                round: None,
                signed: false,
            },
        }
    }

    pub fn round_status(&self, meeting: MeetingId) -> RoundStatus {
        match self.lock().rounds.get(&meeting) {
            Some(r) => RoundStatus {
                round: Some(r.id),
                signed: r.signed.len(),
            },
            None => RoundStatus {
                round: None,
                signed: 0,
            },
        }
    }

    // ── Cleanup ──────────────────────────────────────────────────────────────

    pub fn drop_round(&self, meeting: MeetingId) {
        self.lock().rounds.remove(&meeting);
    }

    pub fn drop_meeting(&self, meeting: MeetingId) {
        let mut inner = self.lock();
        inner.rounds.remove(&meeting);
        inner.sessions.retain(|_, s| s.who.meeting != meeting);
        inner.tickets.retain(|_, t| t.who.meeting != meeting);
    }

    /// Logs a voter out everywhere (they were removed, or their invite was reset).
    pub fn drop_voter(&self, who: Identity) {
        let mut inner = self.lock();
        inner.sessions.retain(|_, s| s.who != who);
        inner.tickets.retain(|_, t| t.who != who);
    }

    /// Forgets expired sessions and tickets, and rounds older than [`ROUND_TTL`].
    pub fn prune(&self, now: Instant) {
        let mut inner = self.lock();
        inner.sessions.retain(|_, s| s.expires > now);
        inner.tickets.retain(|_, t| t.expires > now);
        inner.rounds.retain(|_, r| now.duration_since(r.started) < ROUND_TTL);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;

    fn test_key() -> RoundKey {
        RoundKey::generate().expect("key generation")
    }

    fn who() -> Identity {
        Identity {
            meeting: Uuid::new_v4(),
            voter: Uuid::new_v4(),
        }
    }

    fn logged_in(state: &State, w: Identity, now: Instant) -> String {
        let ticket = new_token();
        state.issue_ticket(&ticket, w, now);
        state.redeem_ticket(&ticket, now).unwrap().0
    }

    #[test]
    fn ticket_logs_in_once() {
        let (state, w, now) = (State::default(), who(), Instant::now());
        let ticket = new_token();
        state.issue_ticket(&ticket, w, now);
        let (token, got) = state.redeem_ticket(&ticket, now).unwrap();
        assert_eq!(got, w);
        assert_eq!(state.session(&token, now), Some(w));
        assert_eq!(state.redeem_ticket(&ticket, now).unwrap_err().code, ErrorCode::TicketInvalid);
    }

    #[test]
    fn ticket_expires() {
        let (state, w, now) = (State::default(), who(), Instant::now());
        let ticket = new_token();
        state.issue_ticket(&ticket, w, now);
        let later = now + Duration::from_secs(TICKET_TTL_SECS + 1);
        assert_eq!(state.redeem_ticket(&ticket, later).unwrap_err().code, ErrorCode::TicketInvalid);
    }

    #[test]
    fn session_expires_and_logout_works() {
        let (state, w, now) = (State::default(), who(), Instant::now());
        let token = logged_in(&state, w, now);
        assert_eq!(state.session(&token, now + SESSION_TTL), None);
        state.logout(&token);
        assert_eq!(state.session(&token, now), None);
    }

    #[test]
    fn one_signature_per_eligible_voter() {
        let (state, w, now) = (State::default(), who(), Instant::now());
        let round = Uuid::new_v4();
        state.start_round(w.meeting, round, vec![w.voter], test_key(), now);

        assert!(state.claim_signature(w, round).is_ok());
        assert_eq!(state.claim_signature(w, round).unwrap_err().code, ErrorCode::AlreadySigned);
        assert_eq!(state.voter_status(w), VoterStatus { round: Some(round), signed: true });
        assert_eq!(state.round_status(w.meeting).signed, 1);
    }

    #[test]
    fn ineligible_voter_is_refused() {
        let (state, w, now) = (State::default(), who(), Instant::now());
        let round = Uuid::new_v4();
        state.start_round(w.meeting, round, vec![Uuid::new_v4()], test_key(), now);
        assert_eq!(state.claim_signature(w, round).unwrap_err().code, ErrorCode::NotEligible);
    }

    #[test]
    fn wrong_or_missing_round_is_refused() {
        let (state, w, now) = (State::default(), who(), Instant::now());
        assert_eq!(
            state.claim_signature(w, Uuid::new_v4()).unwrap_err().code,
            ErrorCode::VotingClosed
        );
        state.start_round(w.meeting, Uuid::new_v4(), vec![w.voter], test_key(), now);
        assert_eq!(
            state.claim_signature(w, Uuid::new_v4()).unwrap_err().code,
            ErrorCode::WrongRound
        );
    }

    #[test]
    fn new_round_resets_signed_set() {
        let (state, w, now) = (State::default(), who(), Instant::now());
        let r1 = Uuid::new_v4();
        state.start_round(w.meeting, r1, vec![w.voter], test_key(), now);
        state.claim_signature(w, r1).unwrap();
        let r2 = Uuid::new_v4();
        state.start_round(w.meeting, r2, vec![w.voter], test_key(), now);
        assert!(state.claim_signature(w, r2).is_ok());
        assert_eq!(state.claim_signature(w, r1).unwrap_err().code, ErrorCode::WrongRound);
    }

    #[test]
    fn release_allows_retry() {
        let (state, w, now) = (State::default(), who(), Instant::now());
        let round = Uuid::new_v4();
        state.start_round(w.meeting, round, vec![w.voter], test_key(), now);
        state.claim_signature(w, round).unwrap();
        state.release_signature(w, round);
        assert!(state.claim_signature(w, round).is_ok());
    }

    #[test]
    fn drop_voter_and_meeting_end_sessions() {
        let (state, now) = (State::default(), Instant::now());
        let a = who();
        let b = Identity { meeting: a.meeting, voter: Uuid::new_v4() };
        let (ta, tb) = (logged_in(&state, a, now), logged_in(&state, b, now));

        state.drop_voter(a);
        assert_eq!(state.session(&ta, now), None);
        assert_eq!(state.session(&tb, now), Some(b));

        state.start_round(a.meeting, Uuid::new_v4(), vec![], test_key(), now);
        state.drop_meeting(a.meeting);
        assert_eq!(state.session(&tb, now), None);
        assert_eq!(state.round_status(a.meeting).round, None);
    }

    #[test]
    fn prune_forgets_stale_rounds() {
        let (state, w, now) = (State::default(), who(), Instant::now());
        state.start_round(w.meeting, Uuid::new_v4(), vec![], test_key(), now);
        state.prune(now + ROUND_TTL);
        assert_eq!(state.round_status(w.meeting).round, None);
    }
}
