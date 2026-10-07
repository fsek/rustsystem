//! One meeting's state and every rule that governs it (`docs/PROTOCOL.md` §4–5).
//!
//! A meeting has a voter list and is always in exactly one [`Phase`]:
//!
//! ```text
//!            start round              close round
//!   Idle ───────────────▶ Voting ───────────────▶ Tallied
//!    ▲                      │                        │
//!    └──── cancel round ────┘◀──────── reset ────────┘
//! ```
//!
//! Every method here is synchronous and either fully succeeds or changes nothing. HTTP handlers
//! hold the meeting's lock, call these, and talk to trustauth; all the rules live here, where
//! they can be tested without a network.

use std::{
    collections::{HashMap, HashSet},
    time::{SystemTime, UNIX_EPOCH},
};

use chrono::{DateTime, Utc};
use rand::seq::SliceRandom;
use serde::Serialize;
use uuid::Uuid;

use rustsystem_core::{
    ApiError, ApiResult, ErrorCode,
    blind::RoundPublicKey,
    internal::{RoundId, VoterId},
    secret::{TokenHash, b64_decode, hash_token, new_token},
};

use crate::{
    agenda::Agenda,
    ballot::{self, RoundRules},
};

// ── Limits (served to the frontend by `GET /api/config`) ─────────────────────

/// Voter, host and candidate names.
pub const MAX_NAME_LENGTH: usize = 80;
/// Meeting titles and round names.
pub const MAX_LABEL_LENGTH: usize = 120;
pub const MAX_CANDIDATES: usize = 100;
pub const MAX_VOTERS: usize = 1000;

/// Trims `value` and checks it is non-empty, short enough, and free of control characters.
pub fn clean_text(field: &str, value: &str, max: usize) -> ApiResult<String> {
    let value = value.trim();
    if value.is_empty() {
        return Err(ApiError::invalid_input(format!("{field} can't be empty.")));
    }
    if value.chars().count() > max {
        return Err(ApiError::invalid_input(format!("{field} can't be longer than {max} characters.")));
    }
    if value.chars().any(char::is_control) {
        return Err(ApiError::invalid_input(format!("{field} contains invalid characters.")));
    }
    Ok(value.to_owned())
}

// ── Voters ───────────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct Voter {
    pub name: String,
    pub is_host: bool,
    pub logged_in: bool,
    /// `SHA-256` of the one-time invite secret, until it is used.
    invite: Option<TokenHash>,
    pub added_at: SystemTime,
}

impl Voter {
    pub fn added_at_unix(&self) -> u64 {
        self.added_at.duration_since(UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0)
    }
}

// ── Rounds ───────────────────────────────────────────────────────────────────

/// A validated description of a round, before it is opened.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RoundSpec {
    pub name: String,
    pub candidates: Vec<String>,
    pub max_choices: usize,
}

impl RoundSpec {
    pub fn new(name: &str, candidates: &[String], max_choices: usize, shuffle: bool) -> ApiResult<Self> {
        let name = clean_text("The round name", name, MAX_LABEL_LENGTH)?;
        if candidates.is_empty() {
            return Err(ApiError::invalid_input("A round needs at least one candidate."));
        }
        if candidates.len() > MAX_CANDIDATES {
            return Err(ApiError::invalid_input(format!("A round can have at most {MAX_CANDIDATES} candidates.")));
        }
        let mut cleaned = candidates
            .iter()
            .map(|c| clean_text("A candidate name", c, MAX_NAME_LENGTH))
            .collect::<ApiResult<Vec<_>>>()?;
        if cleaned.iter().collect::<HashSet<_>>().len() != cleaned.len() {
            return Err(ApiError::invalid_input("Candidate names must be unique."));
        }
        if max_choices == 0 || max_choices > cleaned.len() {
            return Err(ApiError::invalid_input(format!(
                "Max choices must be between 1 and the number of candidates ({}).",
                cleaned.len()
            )));
        }
        if shuffle {
            cleaned.shuffle(&mut rand::rng());
        }
        Ok(Self { name, candidates: cleaned, max_choices })
    }
}

pub struct Round {
    pub id: RoundId,
    pub spec: RoundSpec,
    pub public_key: RoundPublicKey,
    /// SPKI DER, base64url, as sent to browsers.
    pub public_key_b64: String,
    pub eligible: usize,
    pub opened_at: DateTime<Utc>,
    received: HashSet<[u8; 32]>,
    score: Vec<usize>,
    blank: usize,
}

impl Round {
    pub fn new(id: RoundId, spec: RoundSpec, public_key_b64: String, eligible: usize) -> ApiResult<Self> {
        let public_key = b64_decode(&public_key_b64)
            .and_then(|der| RoundPublicKey::from_der(&der).ok())
            .ok_or_else(|| ApiError::internal("trustauth returned an unreadable round key"))?;
        let score = vec![0; spec.candidates.len()];
        Ok(Self {
            id,
            spec,
            public_key,
            public_key_b64,
            eligible,
            opened_at: Utc::now(),
            received: HashSet::new(),
            score,
            blank: 0,
        })
    }

    pub fn received(&self) -> usize {
        self.received.len()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Counts {
    pub eligible: usize,
    /// From trustauth; `None` if it couldn't be reached when the round closed.
    pub signed: Option<usize>,
    pub received: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Tally {
    /// `score[i]` is the number of ballots choosing `candidates[i]`.
    pub score: Vec<usize>,
    pub blank: usize,
}

#[derive(Debug, Clone)]
pub struct ClosedRound {
    pub id: RoundId,
    pub spec: RoundSpec,
    pub tally: Tally,
    pub counts: Counts,
    pub opened_at: DateTime<Utc>,
    pub closed_at: DateTime<Utc>,
}

pub enum Phase {
    Idle,
    Voting(Round),
    Tallied(ClosedRound),
}

impl Phase {
    pub fn name(&self) -> &'static str {
        match self {
            Phase::Idle => "idle",
            Phase::Voting(_) => "voting",
            Phase::Tallied(_) => "tallied",
        }
    }
}

// ── Attendance ───────────────────────────────────────────────────────────────

/// One attendance check: who was logged in, and where on the agenda the meeting was.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Attendance {
    /// RFC 3339, like the times in tally files.
    pub taken_at: String,
    /// The point as it was then. A copy, so later agenda edits don't rewrite the record.
    pub point: Option<AttendancePoint>,
    pub present: Vec<Present>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AttendancePoint {
    pub index: usize,
    pub title: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Present {
    pub id: VoterId,
    pub name: String,
    pub is_host: bool,
}

// ── The meeting ──────────────────────────────────────────────────────────────

pub struct MeetingState {
    voters: HashMap<VoterId, Voter>,
    phase: Phase,
    agenda: Option<Agenda>,
    /// Index into `agenda.points`; meaningless without an agenda.
    current: usize,
    attendance: Vec<Attendance>,
}

impl MeetingState {
    /// A meeting whose only member is its host, already logged in.
    pub fn new(host_name: &str) -> ApiResult<(Self, VoterId)> {
        let name = clean_text("Your name", host_name, MAX_NAME_LENGTH)?;
        let host = Uuid::new_v4();
        let voter = Voter {
            name,
            is_host: true,
            logged_in: true,
            invite: None,
            added_at: SystemTime::now(),
        };
        let state = Self {
            voters: HashMap::from([(host, voter)]),
            phase: Phase::Idle,
            agenda: None,
            current: 0,
            attendance: Vec::new(),
        };
        Ok((state, host))
    }

    pub fn voter(&self, id: VoterId) -> Option<&Voter> {
        self.voters.get(&id)
    }

    /// Voters in the order they were added.
    pub fn voters(&self) -> Vec<(VoterId, &Voter)> {
        let mut list: Vec<_> = self.voters.iter().map(|(id, v)| (*id, v)).collect();
        list.sort_by(|a, b| a.1.added_at.cmp(&b.1.added_at).then(a.1.name.cmp(&b.1.name)));
        list
    }

    pub fn participants(&self) -> usize {
        self.voters.values().filter(|v| v.logged_in).count()
    }

    pub fn phase(&self) -> &Phase {
        &self.phase
    }

    fn ensure_not_voting(&self) -> ApiResult<()> {
        match self.phase {
            Phase::Voting(_) => Err(ErrorCode::RoundInProgress.into()),
            _ => Ok(()),
        }
    }

    fn voter_mut(&mut self, id: VoterId) -> ApiResult<&mut Voter> {
        self.voters.get_mut(&id).ok_or_else(|| ErrorCode::VoterNotFound.into())
    }

    // ── Voter list (frozen while a round is open) ────────────────────────────

    /// Adds a voter and returns their ID and one-time invite secret.
    pub fn add_voter(&mut self, name: &str, is_host: bool) -> ApiResult<(VoterId, String)> {
        self.ensure_not_voting()?;
        let name = clean_text("The name", name, MAX_NAME_LENGTH)?;
        if self.voters.values().any(|v| v.name == name) {
            return Err(ErrorCode::NameTaken.into());
        }
        if self.voters.len() >= MAX_VOTERS {
            return Err(ApiError::invalid_input(format!("A meeting can have at most {MAX_VOTERS} voters.")));
        }
        let id = Uuid::new_v4();
        let invite = new_token();
        self.voters.insert(
            id,
            Voter {
                name,
                is_host,
                logged_in: false,
                invite: Some(hash_token(&invite)),
                added_at: SystemTime::now(),
            },
        );
        Ok((id, invite))
    }

    /// Replaces a voter's invite secret and logs them out. Their ID stays the same, so a reset
    /// can never earn anyone a second signature. The caller must also end their sessions.
    pub fn reset_invite(&mut self, id: VoterId) -> ApiResult<String> {
        self.ensure_not_voting()?;
        let voter = self.voter_mut(id)?;
        let invite = new_token();
        voter.invite = Some(hash_token(&invite));
        voter.logged_in = false;
        Ok(invite)
    }

    pub fn remove_voter(&mut self, actor: VoterId, id: VoterId) -> ApiResult<Voter> {
        self.ensure_not_voting()?;
        if actor == id {
            return Err(ErrorCode::CannotRemoveSelf.into());
        }
        self.voters.remove(&id).ok_or_else(|| ErrorCode::VoterNotFound.into())
    }

    /// Removes everyone who isn't a host and returns who was removed.
    pub fn remove_non_hosts(&mut self) -> ApiResult<Vec<VoterId>> {
        self.ensure_not_voting()?;
        let removed: Vec<_> = self.voters.iter().filter(|(_, v)| !v.is_host).map(|(id, _)| *id).collect();
        for id in &removed {
            self.voters.remove(id);
        }
        Ok(removed)
    }

    // ── Login ────────────────────────────────────────────────────────────────

    /// Finds whose invite this is, without using it up.
    pub fn find_invite(&self, invite: &str) -> ApiResult<VoterId> {
        let hash = hash_token(invite);
        self.voters
            .iter()
            .find(|(_, v)| v.invite == Some(hash))
            .map(|(id, _)| *id)
            .ok_or_else(|| ErrorCode::InviteInvalid.into())
    }

    /// Uses up the voter's invite and marks them logged in.
    pub fn claim_invite(&mut self, id: VoterId) -> ApiResult<()> {
        let voter = self.voter_mut(id)?;
        if voter.invite.take().is_none() {
            return Err(ErrorCode::InviteInvalid.into());
        }
        voter.logged_in = true;
        Ok(())
    }

    // ── Rounds ───────────────────────────────────────────────────────────────

    /// Who would be eligible if a round started now: everyone who has logged in.
    pub fn eligible_voters(&self) -> ApiResult<Vec<VoterId>> {
        if !matches!(self.phase, Phase::Idle) {
            return Err(ErrorCode::RoundInProgress.into());
        }
        Ok(self.voters.iter().filter(|(_, v)| v.logged_in).map(|(id, _)| *id).collect())
    }

    /// Opens `round` (trustauth already holds its key) and removes voters who never logged in.
    /// Returns the removed voters.
    pub fn open_round(&mut self, round: Round) -> ApiResult<Vec<VoterId>> {
        if !matches!(self.phase, Phase::Idle) {
            return Err(ErrorCode::RoundInProgress.into());
        }
        let unclaimed: Vec<_> = self.voters.iter().filter(|(_, v)| !v.logged_in).map(|(id, _)| *id).collect();
        for id in &unclaimed {
            self.voters.remove(id);
        }
        self.phase = Phase::Voting(round);
        Ok(unclaimed)
    }

    /// Counts a ballot, or explains why not (`docs/PROTOCOL.md` §5.3, steps 1–7).
    pub fn accept_ballot(&mut self, prepared: &[u8], sig: &[u8]) -> ApiResult<()> {
        let Phase::Voting(round) = &mut self.phase else {
            return Err(ErrorCode::VotingClosed.into());
        };
        let rules = RoundRules {
            id: round.id,
            public_key: &round.public_key,
            candidates: round.spec.candidates.len(),
            max_choices: round.spec.max_choices,
        };
        let ballot = ballot::check(&rules, prepared, sig)?;
        if round.received.contains(&ballot.hash) {
            return Err(ErrorCode::AlreadyReceived.into());
        }
        if round.received.len() >= round.eligible {
            return Err(ErrorCode::BallotLimitReached.into());
        }
        round.received.insert(ballot.hash);
        match ballot.choice {
            Some(choice) => choice.into_iter().for_each(|c| round.score[c] += 1),
            None => round.blank += 1,
        }
        Ok(())
    }

    /// The result of closing the open round now. Changes nothing: the caller writes the tally
    /// file first and then calls [`finish_close`](Self::finish_close), so a failed write leaves
    /// the round open and no votes are lost.
    pub fn tally(&self, signed: Option<usize>) -> ApiResult<ClosedRound> {
        let Phase::Voting(round) = &self.phase else {
            return Err(ErrorCode::VotingClosed.into());
        };
        Ok(ClosedRound {
            id: round.id,
            spec: round.spec.clone(),
            tally: Tally {
                score: round.score.clone(),
                blank: round.blank,
            },
            counts: Counts {
                eligible: round.eligible,
                signed,
                received: round.received(),
            },
            opened_at: round.opened_at,
            closed_at: Utc::now(),
        })
    }

    pub fn finish_close(&mut self, closed: ClosedRound) -> ApiResult<()> {
        match &self.phase {
            Phase::Voting(round) if round.id == closed.id => {
                self.phase = Phase::Tallied(closed);
                Ok(())
            }
            _ => Err(ErrorCode::VotingClosed.into()),
        }
    }

    /// Back to `Idle`, discarding an open round's ballots or a closed round's result.
    /// Returns whether there was anything to reset.
    pub fn reset_round(&mut self) -> bool {
        !matches!(std::mem::replace(&mut self.phase, Phase::Idle), Phase::Idle)
    }

    // ── Agenda (never blocked by a round) ────────────────────────────────────

    /// The agenda and the index of the current point.
    pub fn agenda(&self) -> Option<(&Agenda, usize)> {
        self.agenda.as_ref().map(|a| (a, self.current))
    }

    /// Sets or replaces the agenda. An edit keeps the meeting on the same point if a point with
    /// that title still exists (the first at or after the old position, else the first before);
    /// otherwise the position is kept as close as the new length allows.
    pub fn set_agenda(&mut self, source: &str) -> ApiResult<()> {
        let new = Agenda::parse(source)?;
        let current = match self.agenda() {
            None => 0,
            Some((old, i)) => {
                let title = &old.points[i].title;
                let after = new.points.iter().skip(i).position(|p| &p.title == title).map(|j| i + j);
                let before = || new.points.iter().take(i).rposition(|p| &p.title == title);
                after.or_else(before).unwrap_or(i.min(new.points.len() - 1))
            }
        };
        self.agenda = Some(new);
        self.current = current;
        Ok(())
    }

    /// Returns whether there was an agenda to clear.
    pub fn clear_agenda(&mut self) -> bool {
        self.current = 0;
        self.agenda.take().is_some()
    }

    /// Moves to point `index`. Absolute rather than "next", so two hosts clicking at once can't
    /// skip a point.
    pub fn go_to(&mut self, index: usize) -> ApiResult<()> {
        let agenda = self.agenda.as_ref().ok_or(ErrorCode::NoAgenda)?;
        if index >= agenda.points.len() {
            return Err(ApiError::invalid_input(format!(
                "The agenda has {} points; there is no point {}.",
                agenda.points.len(),
                index + 1
            )));
        }
        self.current = index;
        Ok(())
    }

    // ── Attendance ───────────────────────────────────────────────────────────

    /// Records everyone logged in right now: invite used, not removed, not reset.
    pub fn take_attendance(&mut self) -> &Attendance {
        let point = self.agenda().map(|(a, i)| AttendancePoint { index: i, title: a.points[i].title.clone() });
        let present = self
            .voters()
            .into_iter()
            .filter(|(_, v)| v.logged_in)
            .map(|(id, v)| Present { id, name: v.name.clone(), is_host: v.is_host })
            .collect();
        self.attendance.push(Attendance { taken_at: Utc::now().to_rfc3339(), point, present });
        self.attendance.last().expect("just pushed")
    }

    /// Every attendance check so far, oldest first.
    pub fn attendance(&self) -> &[Attendance] {
        &self.attendance
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ballot::tests::{message, signed};
    use rustsystem_core::{blind::RoundKey, secret::b64_encode};

    fn spec(n: usize, max: usize) -> RoundSpec {
        let candidates: Vec<String> = (0..n).map(|i| format!("C{i}")).collect();
        RoundSpec::new("Chair", &candidates, max, false).unwrap()
    }

    fn meeting() -> (MeetingState, VoterId) {
        MeetingState::new("Host").unwrap()
    }

    fn joined(state: &mut MeetingState, name: &str) -> VoterId {
        let (id, invite) = state.add_voter(name, false).unwrap();
        assert_eq!(state.find_invite(&invite).unwrap(), id);
        state.claim_invite(id).unwrap();
        id
    }

    /// Opens a round for the current eligible voters and returns its key.
    fn open(state: &mut MeetingState, spec: RoundSpec) -> (RoundKey, RoundId) {
        let key = RoundKey::generate().unwrap();
        let id = Uuid::new_v4();
        let eligible = state.eligible_voters().unwrap().len();
        let pk = b64_encode(key.public_key().to_der().unwrap());
        state.open_round(Round::new(id, spec, pk, eligible).unwrap()).unwrap();
        (key, id)
    }

    fn vote(state: &mut MeetingState, key: &RoundKey, id: RoundId, choice: &str) -> ApiResult<()> {
        let (p, s) = signed(key, &message(id, choice));
        state.accept_ballot(&p, &s)
    }

    fn code<T: std::fmt::Debug>(r: ApiResult<T>) -> ErrorCode {
        r.unwrap_err().code
    }

    // ── Voters ───────────────────────────────────────────────────────────────

    #[test]
    fn host_starts_logged_in() {
        let (state, host) = meeting();
        let v = state.voter(host).unwrap();
        assert!(v.is_host && v.logged_in);
        assert_eq!(state.participants(), 1);
    }

    #[test]
    fn names_are_validated() {
        let (mut state, _) = meeting();
        assert_eq!(code(state.add_voter("  ", false)), ErrorCode::InvalidInput);
        assert_eq!(code(state.add_voter(&"x".repeat(MAX_NAME_LENGTH + 1), false)), ErrorCode::InvalidInput);
        assert_eq!(code(state.add_voter("a\u{0}b", false)), ErrorCode::InvalidInput);
        assert_eq!(code(state.add_voter(" Host ", false)), ErrorCode::NameTaken);
        assert!(state.add_voter(&"x".repeat(MAX_NAME_LENGTH), false).is_ok());
    }

    #[test]
    fn invite_works_once() {
        let (mut state, _) = meeting();
        let (id, invite) = state.add_voter("Anna", false).unwrap();
        assert!(!state.voter(id).unwrap().logged_in);
        state.claim_invite(state.find_invite(&invite).unwrap()).unwrap();
        assert!(state.voter(id).unwrap().logged_in);
        assert_eq!(code(state.find_invite(&invite)), ErrorCode::InviteInvalid);
        assert_eq!(code(state.claim_invite(id)), ErrorCode::InviteInvalid);
        assert_eq!(code(state.find_invite("made-up")), ErrorCode::InviteInvalid);
    }

    #[test]
    fn reset_invite_keeps_identity_and_kills_old_link() {
        let (mut state, _) = meeting();
        let (id, old) = state.add_voter("Anna", true).unwrap();
        state.claim_invite(id).unwrap();
        let new = state.reset_invite(id).unwrap();
        assert_eq!(code(state.find_invite(&old)), ErrorCode::InviteInvalid);
        assert_eq!(state.find_invite(&new).unwrap(), id, "same voter ID after reset");
        let v = state.voter(id).unwrap();
        assert!(!v.logged_in && v.is_host, "reset logs out but keeps host status");
    }

    #[test]
    fn hosts_cannot_remove_themselves() {
        let (mut state, host) = meeting();
        let (other_host, _) = state.add_voter("Co-host", true).unwrap();
        assert_eq!(code(state.remove_voter(host, host)), ErrorCode::CannotRemoveSelf);
        assert!(state.remove_voter(host, other_host).is_ok());
        assert_eq!(code(state.remove_voter(host, other_host)), ErrorCode::VoterNotFound);
    }

    #[test]
    fn remove_non_hosts_keeps_hosts() {
        let (mut state, host) = meeting();
        let a = joined(&mut state, "A");
        let (h2, _) = state.add_voter("H2", true).unwrap();
        let removed = state.remove_non_hosts().unwrap();
        assert_eq!(removed, vec![a]);
        assert!(state.voter(host).is_some() && state.voter(h2).is_some());
    }

    #[test]
    fn voter_list_is_frozen_while_voting() {
        let (mut state, host) = meeting();
        let a = joined(&mut state, "A");
        open(&mut state, spec(2, 1));
        assert_eq!(code(state.add_voter("B", false)), ErrorCode::RoundInProgress);
        assert_eq!(code(state.reset_invite(a)), ErrorCode::RoundInProgress);
        assert_eq!(code(state.remove_voter(host, a)), ErrorCode::RoundInProgress);
        assert_eq!(code(state.remove_non_hosts()), ErrorCode::RoundInProgress);
    }

    // ── Round specs ──────────────────────────────────────────────────────────

    #[test]
    fn round_spec_rules() {
        let c = |v: &[&str]| v.iter().map(|s| s.to_string()).collect::<Vec<_>>();
        assert!(RoundSpec::new("R", &c(&["A", "B"]), 2, false).is_ok());
        assert!(RoundSpec::new("", &c(&["A"]), 1, false).is_err());
        assert!(RoundSpec::new("R", &[], 1, false).is_err());
        assert!(RoundSpec::new("R", &c(&["A", " A "]), 1, false).is_err(), "duplicates after trimming");
        assert!(RoundSpec::new("R", &c(&["A", " "]), 1, false).is_err(), "blank candidate");
        assert!(RoundSpec::new("R", &c(&["A"]), 0, false).is_err());
        assert!(RoundSpec::new("R", &c(&["A"]), 2, false).is_err());
        let many: Vec<String> = (0..=MAX_CANDIDATES).map(|i| i.to_string()).collect();
        assert!(RoundSpec::new("R", &many, 1, false).is_err());
    }

    #[test]
    fn shuffle_keeps_the_same_candidates() {
        let candidates: Vec<String> = (0..20).map(|i| i.to_string()).collect();
        let s = RoundSpec::new("R", &candidates, 1, true).unwrap();
        let mut sorted = s.candidates.clone();
        sorted.sort_by_key(|c| c.parse::<u32>().unwrap());
        assert_eq!(sorted, candidates);
    }

    // ── Rounds ───────────────────────────────────────────────────────────────

    #[test]
    fn opening_removes_unclaimed_voters() {
        let (mut state, host) = meeting();
        let a = joined(&mut state, "A");
        let (never, _) = state.add_voter("Never", false).unwrap();
        assert_eq!(state.eligible_voters().unwrap().len(), 2);
        open(&mut state, spec(2, 1));
        assert!(state.voter(never).is_none());
        assert!(state.voter(a).is_some() && state.voter(host).is_some());
        assert_eq!(state.phase().name(), "voting");
    }

    #[test]
    fn only_one_round_at_a_time() {
        let (mut state, _) = meeting();
        open(&mut state, spec(2, 1));
        assert_eq!(code(state.eligible_voters()), ErrorCode::RoundInProgress);
    }

    #[test]
    fn ballots_are_counted() {
        let (mut state, _) = meeting();
        joined(&mut state, "A");
        joined(&mut state, "B");
        let (key, id) = open(&mut state, spec(3, 2));
        vote(&mut state, &key, id, "[0,2]").unwrap();
        vote(&mut state, &key, id, "[2]").unwrap();
        vote(&mut state, &key, id, "null").unwrap();
        let closed = state.tally(Some(3)).unwrap();
        assert_eq!(closed.tally, Tally { score: vec![1, 0, 2], blank: 1 });
        assert_eq!(closed.counts, Counts { eligible: 3, signed: Some(3), received: 3 });
    }

    #[test]
    fn same_ballot_is_counted_once() {
        let (mut state, _) = meeting();
        joined(&mut state, "A");
        let (key, id) = open(&mut state, spec(2, 1));
        let (p, s) = signed(&key, &message(id, "[0]"));
        state.accept_ballot(&p, &s).unwrap();
        assert_eq!(code(state.accept_ballot(&p, &s)), ErrorCode::AlreadyReceived);
        assert_eq!(state.tally(None).unwrap().tally.score, vec![1, 0]);
    }

    #[test]
    fn never_more_ballots_than_eligible_voters() {
        let (mut state, _) = meeting(); // just the host: one eligible voter
        let (key, id) = open(&mut state, spec(2, 1));
        vote(&mut state, &key, id, "[0]").unwrap();
        assert_eq!(code(vote(&mut state, &key, id, "[1]")), ErrorCode::BallotLimitReached);
    }

    #[test]
    fn invalid_ballots_change_nothing() {
        let (mut state, _) = meeting();
        let (key, id) = open(&mut state, spec(2, 1));
        assert_eq!(code(vote(&mut state, &key, id, "[0,0]")), ErrorCode::InvalidBallot);
        assert_eq!(code(vote(&mut state, &key, id, "[5]")), ErrorCode::InvalidBallot);
        let closed = state.tally(None).unwrap();
        assert_eq!(closed.counts.received, 0);
        assert_eq!(closed.tally, Tally { score: vec![0, 0], blank: 0 });
    }

    #[test]
    fn ballots_need_an_open_round() {
        let (mut state, _) = meeting();
        let key = RoundKey::generate().unwrap();
        assert_eq!(code(vote(&mut state, &key, Uuid::new_v4(), "[0]")), ErrorCode::VotingClosed);
    }

    #[test]
    fn previous_rounds_ballots_are_rejected() {
        let (mut state, _) = meeting();
        let (old_key, old_id) = open(&mut state, spec(2, 1));
        state.reset_round();
        open(&mut state, spec(2, 1));
        assert_eq!(code(vote(&mut state, &old_key, old_id, "[0]")), ErrorCode::InvalidSignature);
    }

    #[test]
    fn tally_changes_nothing_until_finished() {
        let (mut state, _) = meeting();
        let (key, id) = open(&mut state, spec(2, 1));
        let closed = state.tally(None).unwrap();
        // e.g. writing the tally file failed: the round is still open and still counting.
        assert_eq!(state.phase().name(), "voting");
        vote(&mut state, &key, id, "[1]").unwrap();
        state.finish_close(closed).unwrap();
        assert_eq!(state.phase().name(), "tallied");
        assert_eq!(code(vote(&mut state, &key, id, "[1]")), ErrorCode::VotingClosed);
    }

    #[test]
    fn finish_close_needs_the_same_round() {
        let (mut state, _) = meeting();
        open(&mut state, spec(2, 1));
        let stale = state.tally(None).unwrap();
        state.reset_round();
        open(&mut state, spec(2, 1));
        assert_eq!(code(state.finish_close(stale)), ErrorCode::VotingClosed);
    }

    #[test]
    fn reset_returns_to_idle() {
        let (mut state, _) = meeting();
        assert!(!state.reset_round());
        open(&mut state, spec(2, 1));
        assert!(state.reset_round());
        assert_eq!(state.phase().name(), "idle");
        open(&mut state, spec(2, 1));
        let closed = state.tally(None).unwrap();
        state.finish_close(closed).unwrap();
        assert!(state.reset_round());
        assert!(state.eligible_voters().is_ok());
    }

    // ── Agenda ───────────────────────────────────────────────────────────────

    const AGENDA: &str = "# Opening\n# Budget\n## Vote\n# Closing\n";

    fn current_title(state: &MeetingState) -> &str {
        let (a, i) = state.agenda().unwrap();
        &a.points[i].title
    }

    #[test]
    fn moving_needs_an_agenda_and_a_real_point() {
        let (mut state, _) = meeting();
        assert_eq!(code(state.go_to(0)), ErrorCode::NoAgenda);
        state.set_agenda(AGENDA).unwrap();
        assert_eq!(current_title(&state), "Opening");
        state.go_to(3).unwrap();
        state.go_to(2).unwrap(); // back
        assert_eq!(current_title(&state), "Vote");
        assert_eq!(code(state.go_to(4)), ErrorCode::InvalidInput);
        assert_eq!(current_title(&state), "Vote", "a failed move changes nothing");
    }

    #[test]
    fn agenda_works_during_a_round() {
        let (mut state, _) = meeting();
        open(&mut state, spec(2, 1));
        state.set_agenda(AGENDA).unwrap();
        state.go_to(1).unwrap();
    }

    #[test]
    fn editing_keeps_the_current_point_by_title() {
        let (mut state, _) = meeting();
        state.set_agenda(AGENDA).unwrap();
        state.go_to(1).unwrap(); // Budget
        state.set_agenda("# Opening\n# Minutes\n# Budget\n# Closing\n").unwrap();
        assert_eq!(current_title(&state), "Budget", "a point inserted before it");
        state.set_agenda("# Budget\n# Closing\n").unwrap();
        assert_eq!(current_title(&state), "Budget", "points removed before it");
        state.go_to(1).unwrap(); // Closing
        state.set_agenda("# Opening\n# Ending\n").unwrap();
        assert_eq!(state.agenda().unwrap().1, 1, "renamed: the position is kept");
        state.set_agenda("# Only\n").unwrap();
        assert_eq!(state.agenda().unwrap().1, 0, "clamped to the new length");
    }

    #[test]
    fn a_bad_edit_changes_nothing() {
        let (mut state, _) = meeting();
        state.set_agenda(AGENDA).unwrap();
        state.go_to(2).unwrap();
        assert_eq!(code(state.set_agenda("no headings")), ErrorCode::InvalidInput);
        assert_eq!(current_title(&state), "Vote");
        assert!(state.clear_agenda());
        assert!(state.agenda().is_none());
        assert!(!state.clear_agenda());
    }

    // ── Attendance ───────────────────────────────────────────────────────────

    fn names(a: &Attendance) -> Vec<&str> {
        a.present.iter().map(|p| p.name.as_str()).collect()
    }

    #[test]
    fn attendance_is_everyone_logged_in() {
        let (mut state, _) = meeting();
        let alice = joined(&mut state, "Alice");
        let bob = joined(&mut state, "Bob");
        state.add_voter("Never joined", false).unwrap();
        assert_eq!(names(state.take_attendance()), ["Host", "Alice", "Bob"]);

        state.remove_voter(alice, bob).unwrap();
        state.reset_invite(alice).unwrap();
        let second = state.take_attendance();
        assert_eq!(names(second), ["Host"], "removed and reset voters are not logged in");
        assert!(second.present[0].is_host);
        assert_eq!(state.attendance().len(), 2);
    }

    #[test]
    fn attendance_records_the_point_and_keeps_it() {
        let (mut state, _) = meeting();
        assert_eq!(state.take_attendance().point, None, "allowed without an agenda");
        state.set_agenda(AGENDA).unwrap();
        state.go_to(1).unwrap();
        state.take_attendance();
        state.set_agenda("# Something else\n").unwrap();
        let point = state.attendance()[1].point.clone().unwrap();
        assert_eq!((point.index, point.title.as_str()), (1, "Budget"), "edits don't rewrite history");
    }
}
