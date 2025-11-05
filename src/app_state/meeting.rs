use std::{
    collections::{HashMap, HashSet},
    sync::{Arc, Mutex},
    time::{Duration, SystemTime},
};

use serde::{Deserialize, Serialize};
use uuid::Uuid;

pub const INVITE_TTL: Duration = Duration::from_secs(30);

#[derive(Debug, Clone)]
pub struct Meeting {
    inner: Arc<Mutex<MeetingInner>>,
    admin_token: Uuid,
}

#[derive(Debug, Default)]
struct MeetingInner {
    voters: HashSet<Uuid>,
    /// Map of invitation IDs to their creation times
    invitations: HashMap<Uuid, SystemTime>,
    current_voting: Option<Voting>,
    past_votings: Vec<VotingResults>,
}

impl Meeting {
    pub fn new() -> Self {
        Self {
            admin_token: Uuid::new_v4(),
            inner: Default::default(),
        }
    }

    pub fn create_invitation(&mut self) -> Uuid {
        let invitation_id = Uuid::new_v4();
        self.inner
            .lock()
            .unwrap()
            .invitations
            .insert(invitation_id, SystemTime::now());
        invitation_id
    }

    pub fn use_invitation(&mut self, invitation_id: &Uuid) -> Option<Uuid> {
        let mut inner = self.inner.lock().unwrap();

        if let Some(created_at) = inner.invitations.remove(invitation_id)
            && created_at.elapsed().unwrap() < INVITE_TTL
        {
            let voter_id = Uuid::new_v4();
            inner.voters.insert(voter_id);
            Some(voter_id)
        } else {
            // Invitation is either invalid or expired
            None
        }
    }

    pub fn admin_token(&self) -> Uuid {
        self.admin_token
    }

    pub fn contains_voter(&self, voter_id: &Uuid) -> bool {
        self.inner.lock().unwrap().voters.contains(voter_id)
    }

    pub fn start_voting(&self, info: VotingInfo) {
        let mut inner = self.inner.lock().unwrap();

        let new_voting = Voting {
            info,
            ballots: inner
                .voters
                .iter()
                .map(|voter_id| (*voter_id, None))
                .collect(),
        };

        let previous_voting = inner.current_voting.replace(new_voting);

        if let Some(voting) = previous_voting {
            inner.past_votings.push(voting.into_results());
        }
    }

    pub fn stop_voting(&self) {
        let mut inner = self.inner.lock().unwrap();

        if let Some(voting) = inner.current_voting.take() {
            inner.past_votings.push(voting.into_results());
        }
    }

    pub fn current_voting(&self) -> Option<VotingInfo> {
        self.inner
            .lock()
            .unwrap()
            .current_voting
            .as_ref()
            .map(|voting| voting.info.clone())
    }

    pub fn cast_vote(&self, voter_id: &Uuid, ballot: Ballot) -> Result<(), CastVoteError> {
        if let Some(current_voting) = &mut self.inner.lock().unwrap().current_voting {
            current_voting.cast_vote(voter_id, ballot)
        } else {
            Err(CastVoteError::VoterNotFound)
        }
    }

    pub fn past_votings(&self) -> Vec<VotingResults> {
        self.inner.lock().unwrap().past_votings.clone()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VotingInfo {
    pub title: String,
    pub options: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Ballot {
    /// An invalid index indicates a blank vote
    pub option: usize,
}

#[derive(Debug)]
struct Voting {
    info: VotingInfo,
    /// Map of voter IDs to their ballots. Do not insert new entries, only
    /// update existing ones to avoid unauthorized voters.
    ballots: HashMap<Uuid, Option<Ballot>>,
}

#[derive(Debug, Clone)]
pub struct VotingResults {
    pub title: String,
    pub option_tallies: Vec<(String, usize)>,
    pub registered_voters: usize,
    pub votes_cast: usize,
}

pub enum CastVoteError {
    VoterNotFound,
    AlreadyVoted,
}

impl Voting {
    pub fn cast_vote(&mut self, voter_id: &Uuid, ballot: Ballot) -> Result<(), CastVoteError> {
        let entry = self
            .ballots
            .get_mut(voter_id)
            .ok_or(CastVoteError::VoterNotFound)?;

        if entry.is_some() {
            Err(CastVoteError::AlreadyVoted)
        } else {
            *entry = Some(ballot);
            Ok(())
        }
    }

    pub fn into_results(self) -> VotingResults {
        let mut option_tallies = self
            .info
            .options
            .into_iter()
            .map(|option| (option.clone(), 0))
            .collect::<Vec<_>>();

        for ballot in self.ballots.values().filter_map(|b| b.as_ref()) {
            if let Some(tally) = option_tallies.get_mut(ballot.option) {
                tally.1 += 1;
            }
        }

        VotingResults {
            title: self.info.title,
            option_tallies,
            registered_voters: self.ballots.len(),
            votes_cast: self
                .ballots
                .values()
                .filter(|ballot_opt| ballot_opt.is_some())
                .count(),
        }
    }
}
