use api_derive::APIEndpointError;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use tokio::sync::watch::{Receiver, Sender};
use zkryptium::{
    keys::pair::KeyPair,
    schemes::{algorithms::BbsBls12381Sha256, generics::BlindSignature},
};

use api_core::{APIErrorCode, APIResult};
use rustsystem_proof::{BallotMetaData, CandidateID, Choice, Provider, Sha256Provider, VoteMethod};

use crate::UUID;

pub type AuthenticationKeys = KeyPair<BbsBls12381Sha256>;

pub type Header = Vec<u8>;

type Votes = Vec<Option<Choice>>;

// The structure of the Tally depends on the voting method
#[derive(Serialize, Deserialize, Debug)]
pub enum TallyScore {
    // Votes for "Yes" - Votes for "No"
    Dichotomous(usize, usize),

    // Votes for each candidate
    Plurality(HashMap<CandidateID, usize>),

    // Score for each candidate
    RankedChoice(HashMap<CandidateID, usize>),

    // Number of approvals for each candidate
    Approval(HashMap<CandidateID, usize>),

    // Total score for each candidate
    Score(HashMap<CandidateID, usize>),

    // Total score for each candidate
    STAR(HashMap<CandidateID, usize>),
}

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

    #[api(code = APIErrorCode::MUIDNotFound, status = 404)]
    MUIDNotFound,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Tally {
    pub score: TallyScore,
    pub blank: usize,
}
impl Tally {
    fn tally_dichotomous(votes: Votes) -> TallyResult<Self> {
        let mut yes_votes = 0;
        let mut no_votes = 0;
        let mut blank_votes = 0;

        for vote in votes {
            match vote {
                Some(choice) => match choice {
                    Choice::Dichotomous(v) => {
                        if v {
                            yes_votes += 1;
                        } else {
                            no_votes += 1;
                        }
                    }
                    _ => {
                        return Err(TallyError::InvalidVoteMethod);
                    }
                },
                None => {
                    blank_votes += 1;
                }
            }
        }

        Ok(Self {
            score: TallyScore::Dichotomous(yes_votes, no_votes),
            blank: blank_votes,
        })
    }
}

pub struct VoteRound {
    metadata: BallotMetaData,
    keys: AuthenticationKeys,
    header: Header,
    registered_voters: HashSet<UUID>,
    expired_signatures: HashSet<[u8; 80]>,

    votes: Votes,
}
impl VoteRound {
    pub fn keys(&self) -> &AuthenticationKeys {
        &self.keys
    }

    pub fn metadata(&self) -> BallotMetaData {
        self.metadata
    }

    pub fn register_user(&mut self, uuid: UUID) {
        self.registered_voters.insert(uuid);
    }

    /// Checks if a user has already registered for voting
    pub fn is_registered(&self, uuid: UUID) -> bool {
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
        match self.metadata.get_method() {
            VoteMethod::Dichotomous => Tally::tally_dichotomous(votes),
            _ => unimplemented!(),
        }
    }
}

pub struct VoteAuthority {
    state_tx: Sender<bool>,
    round: Option<VoteRound>,
}
impl VoteAuthority {
    /// For new meeting
    pub fn new() -> Self {
        Self {
            state_tx: Sender::new(false),
            round: None,
        }
    }

    pub fn is_active(&self) -> bool {
        *self.state_tx.borrow()
    }

    pub fn start_round(&mut self, metadata: BallotMetaData, header: String) {
        let keys = Sha256Provider::generate_authentication_keys();
        let header = header.as_bytes().to_vec();
        let registered_voters = HashSet::new();
        let expired_signatures = HashSet::new();
        self.state_tx.send(true);
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
        self.state_tx.send(false);
        self.round
            .take()
            .ok_or_else(|| TallyError::VotingInactive)?
            .tally()
    }

    pub fn new_watcher(&self) -> Receiver<bool> {
        self.state_tx.subscribe()
    }
}
