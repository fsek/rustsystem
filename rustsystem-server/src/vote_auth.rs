use rustsystem_core::APIError;
use rustsystem_core::APIErrorCode;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use tokio::sync::watch::{Receiver, Sender};
use tracing::error;
use tracing::warn;
use zkryptium::{
    bbsplus::keys::BBSplusPublicKey,
    schemes::{algorithms::BbsBls12381Sha256, generics::BlindSignature},
};

use crate::proof::{BallotMetaData, Choice};

pub type Header = Vec<u8>;

type Votes = Vec<Option<Choice>>;

pub type TallyScore = HashMap<String, usize>;

#[derive(Serialize, Deserialize, Debug)]
pub struct Tally {
    pub score: TallyScore,
    pub blank: usize,
}
impl Tally {
    fn compute(votes: Votes, candidates: &Vec<String>) -> Result<Self, APIError> {
        let mut blank_votes = 0;

        let mut score = HashMap::new();
        for candidate in candidates {
            score.insert(candidate.to_owned(), 0);
        }

        for vote in votes {
            if let Some(choice) = vote {
                for candidate_id in choice {
                    if let Some(candidate) = candidates.get(candidate_id) {
                        if let Some(current_votes) = score.get_mut(candidate) {
                            *current_votes += 1;
                        } else {
                            // This should never fail!
                            warn!("Valid candidate missing in scoring map!");
                        }
                    } else {
                        warn!("Vote contains invalid candidate id: {candidate_id}");
                    }
                }
            } else {
                blank_votes += 1;
            }
        }

        Ok(Self {
            score,
            blank: blank_votes,
        })
    }
}

impl Clone for Tally {
    fn clone(&self) -> Self {
        Self {
            score: self.score.clone(),
            blank: self.blank,
        }
    }
}

pub struct VoteRound {
    metadata: BallotMetaData,
    public_key: BBSplusPublicKey,
    header: Header,
    expired_signatures: HashSet<[u8; 80]>,
    votes: Votes,
}
impl VoteRound {
    pub fn public_key(&self) -> &BBSplusPublicKey {
        &self.public_key
    }

    pub fn metadata(&self) -> BallotMetaData {
        self.metadata.clone()
    }

    pub fn is_used(&self, signature: &BlindSignature<BbsBls12381Sha256>) -> bool {
        self.expired_signatures.contains(&signature.to_bytes())
    }

    pub fn set_signature_expired(&mut self, signature: &BlindSignature<BbsBls12381Sha256>) {
        self.expired_signatures.insert(signature.to_bytes());
    }

    pub fn header(&self) -> &Header {
        &self.header
    }

    pub fn add_vote(&mut self, choice: Option<Choice>) {
        self.votes.push(choice);
    }

    pub fn get_vote_count(&self) -> usize {
        self.votes.len()
    }

    pub fn tally(self) -> Result<Tally, APIError> {
        let votes = self.votes.clone();
        Tally::compute(votes, &self.metadata.get_candidates())
    }
}

#[derive(Clone, PartialEq, PartialOrd, Debug)]
pub enum VoteState {
    Creation,
    Voting,
    Tally,
}

pub struct VoteAuthority {
    state_tx: Sender<VoteState>,
    _state_rx: Receiver<VoteState>,
    update_tx: Sender<bool>,
    round: Option<VoteRound>,
    last_tally: Option<Tally>,
    current_vote_name: Option<String>,
}
impl Default for VoteAuthority {
    fn default() -> Self {
        Self::new()
    }
}

impl VoteAuthority {
    /// For new meeting
    pub fn new() -> Self {
        let state_tx = Sender::new(VoteState::Creation);

        Self {
            // This is to make sure that there is at least one subscriber to state_tx
            _state_rx: state_tx.subscribe(),
            state_tx,
            update_tx: Sender::new(true),
            round: None,
            last_tally: None,
            current_vote_name: None,
        }
    }

    pub fn is_active(&self) -> bool {
        *self.state_tx.borrow() == VoteState::Voting
    }

    pub fn is_tally(&self) -> bool {
        *self.state_tx.borrow() == VoteState::Tally
    }

    pub fn is_inactive(&self) -> bool {
        *self.state_tx.borrow() == VoteState::Creation
    }

    pub fn start_round(&mut self, metadata: BallotMetaData, header: String, public_key: BBSplusPublicKey) {
        let header_bytes = header.as_bytes().to_vec();
        let expired_signatures = HashSet::new();
        if let Err(e) = self.state_tx.send(VoteState::Voting) {
            error!("{e}");
        }
        self.current_vote_name = Some(header.clone());
        self.round = Some(VoteRound {
            public_key,
            header: header_bytes,
            expired_signatures,
            metadata,
            votes: Vec::new(),
        });
    }

    pub fn round(&mut self) -> Option<&mut VoteRound> {
        self.round.as_mut()
    }

    pub fn round_ref(&self) -> Option<&VoteRound> {
        self.round.as_ref()
    }

    // This is the function that should later handle the tallying of votes
    pub fn finalize_round(&mut self) -> Result<Tally, APIError> {
        let res = self
            .round
            .take()
            .ok_or(APIError::from_error_code(APIErrorCode::VotingInactive))?
            .tally();
        if let Err(e) = self.state_tx.send(VoteState::Tally) {
            error!("{e}");
        }
        if let Ok(ref tally) = res {
            self.last_tally = Some(tally.clone());
        }
        res
    }

    // Set everything back to default
    pub fn reset(&mut self) {
        if let Err(e) = self.state_tx.send(VoteState::Creation) {
            error!("{e}");
        }
        self.round = None;
        self.last_tally = None;
        self.current_vote_name = None;
    }

    pub fn new_state_watcher(&self) -> Receiver<VoteState> {
        self.state_tx.subscribe()
    }

    pub fn new_update_watcher(&self) -> Receiver<bool> {
        self.update_tx.subscribe()
    }

    pub fn send_update(&self) {
        self.update_tx.send(true).ok();
    }

    pub fn get_last_tally(&self) -> Option<&Tally> {
        self.last_tally.as_ref()
    }

    pub fn get_current_vote_name(&self) -> Option<&String> {
        self.current_vote_name.as_ref()
    }
}
