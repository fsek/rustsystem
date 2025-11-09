use api_derive::APIEndpointError;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use tokio::sync::watch::{Receiver, Sender};
use tracing::error;
use tracing::warn;
use zkryptium::{
    keys::pair::KeyPair,
    schemes::{algorithms::BbsBls12381Sha256, generics::BlindSignature},
};

use rand::{rng, seq::SliceRandom};

use api_core::{APIErrorCode, APIResult};
use rustsystem_proof::{BallotMetaData, Choice, Provider, Sha256Provider};

use crate::UUuid;

pub type AuthenticationKeys = KeyPair<BbsBls12381Sha256>;

pub type Header = Vec<u8>;

type Votes = Vec<Option<Choice>>;

pub type TallyScore = HashMap<String, usize>;

pub type TallyResult<T> = APIResult<T, TallyError>;

#[derive(APIEndpointError, Debug)]
#[api(endpoint(method = "POST", path = "/api/host/tally"))]
pub enum TallyError {
    // If an invalid vote has gotten to the point of tallying, there is something wrong inside of
    // the server. This should NEVER happen. Invalid methods should be checked upon receival.
    #[api(code = APIErrorCode::InvalidVoteMethod, status = 500)]
    InvalidVoteMethod,
    #[api(code = APIErrorCode::VotingInactive, status = 410)]
    VotingInactive,

    #[api(code = APIErrorCode::MUuidNotFound, status = 404)]
    MUIDNotFound,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Tally {
    pub score: TallyScore,
    pub blank: usize,
}
impl Tally {
    fn tally(votes: Votes, candidates: &Vec<String>) -> TallyResult<Self> {
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

pub struct VoteRound {
    metadata: BallotMetaData,
    keys: AuthenticationKeys,
    header: Header,
    registered_voters: HashSet<UUuid>,
    expired_signatures: HashSet<[u8; 80]>,
    votes: Votes,
}
impl VoteRound {
    pub fn keys(&self) -> &AuthenticationKeys {
        &self.keys
    }

    pub fn metadata(&self) -> BallotMetaData {
        self.metadata.clone()
    }

    pub fn register_user(&mut self, uuid: UUuid) {
        self.registered_voters.insert(uuid);
    }

    /// Checks if a user has already registered for voting
    pub fn is_registered(&self, uuid: UUuid) -> bool {
        self.registered_voters.contains(&uuid)
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

    pub fn tally(self) -> TallyResult<Tally> {
        let votes = self.votes.clone();
        Tally::tally(votes, &self.metadata.get_candidates())
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

    pub fn start_round(&mut self, mut metadata: BallotMetaData, shuffle: bool, header: String) {
        if shuffle {
            let mut candidates = metadata.get_candidates();
            candidates.shuffle(&mut rng());
            metadata.set_candidates(candidates);
        }

        let keys = Sha256Provider::generate_authentication_keys();
        let header = header.as_bytes().to_vec();
        let registered_voters = HashSet::new();
        let expired_signatures = HashSet::new();
        if let Err(e) = self.state_tx.send(VoteState::Voting) {
            error!("{e}");
        }
        self.round = Some(VoteRound {
            keys,
            header,
            registered_voters,
            expired_signatures,
            metadata,
            votes: Vec::new(),
        });
    }

    pub fn round(&mut self) -> Option<&mut VoteRound> {
        self.round.as_mut()
    }

    // This is the function that should later handle the tallying of votes
    pub fn finalize_round(&mut self) -> TallyResult<Tally> {
        let res = self.round.take().ok_or(TallyError::VotingInactive)?.tally();
        if let Err(e) = self.state_tx.send(VoteState::Tally) {
            error!("{e}");
        }
        res
    }

    // Set everything back to default
    pub fn reset(&mut self) {
        if let Err(e) = self.state_tx.send(VoteState::Creation) {
            error!("{e}");
        }
        self.round = None;
    }

    pub fn new_state_watcher(&self) -> Receiver<VoteState> {
        self.state_tx.subscribe()
    }

    pub fn new_update_watcher(&self) -> Receiver<bool> {
        self.update_tx.subscribe()
    }
}
