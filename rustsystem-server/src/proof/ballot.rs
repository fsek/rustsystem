use std::{
    collections::HashSet,
    io::{self, Error, ErrorKind},
};

use crate::proof::{Sha256ValidationInfo, ValidationInfo};

use rand::Rng;
use serde::{Deserialize, Serialize};
use zkryptium::schemes::{algorithms::BbsBls12381Sha256, generics::BlindSignature};

pub type VoteRoundID = u128;
pub type CandidateID = usize;
pub type ProtocolVersion = u8;

pub type Candidates = Vec<String>;

pub type Choice = Vec<CandidateID>;

#[derive(Serialize, Deserialize, Clone, Debug)]
// All fields are private since they should not change once set
pub struct BallotValidation {
    proof: Vec<u8>,
    token: Vec<u8>,
    signature: BlindSignature<BbsBls12381Sha256>,
}

impl BallotValidation {
    pub fn new(
        proof: Vec<u8>,
        token: Vec<u8>,
        signature: BlindSignature<BbsBls12381Sha256>,
    ) -> Self {
        Self {
            proof,
            token,
            signature,
        }
    }

    // Getter functions for private fields
    pub fn get_proof(&self) -> &Vec<u8> {
        &self.proof
    }
    pub fn get_token(&self) -> &Vec<u8> {
        &self.token
    }
    pub fn get_signature(&self) -> &BlindSignature<BbsBls12381Sha256> {
        &self.signature
    }
}
impl From<BallotValidation> for Sha256ValidationInfo {
    fn from(value: BallotValidation) -> Self {
        Self::new(value.proof, value.token, value.signature)
    }
}

#[derive(Serialize, Deserialize, PartialEq, Eq, Clone, Debug)]
// All fields are private since they should not change once set
pub struct BallotMetaData {
    candidates: Candidates,
    max_choices: usize,
    protocol_version: ProtocolVersion,
}
impl BallotMetaData {
    pub fn new(
        candidates: Candidates,
        protocol_version: ProtocolVersion,
        max_choices: usize,
    ) -> Self {
        Self {
            candidates,
            max_choices,
            protocol_version,
        }
    }

    pub fn debug(&self) -> String {
        format!("{self:?}")
    }
}
impl BallotMetaData {
    // Getter functions for private fields
    pub fn get_candidates(&self) -> Candidates {
        self.candidates.clone()
    }
    pub fn get_protocol_version(&self) -> ProtocolVersion {
        self.protocol_version
    }
    pub fn get_max_choices(&self) -> usize {
        self.max_choices
    }

    pub fn set_candidates(&mut self, new_candidates: Candidates) {
        self.candidates = new_candidates;
    }

    pub fn check_valid(&self) -> bool {
        let set: HashSet<_> = self.candidates.iter().collect();
        if set.len() != self.candidates.len() {
            return false;
        }

        if self.max_choices > self.candidates.len() {
            return false;
        }

        true
    }
}

// Enforce the ballot size (in bytes) to be a certain number.
// It doesn't matter what this number is, but it must be the same for all Ballots.
// Prevents some sophisticated network analysis attacks
const BALLOT_SIZE: usize = 1024;

#[derive(Serialize, Deserialize)]
pub struct Ballot {
    metadata: BallotMetaData,
    choice: Option<Choice>, // None for blank vote
    validation: BallotValidation,
    _padding: Vec<u8>,
}
impl Ballot {
    pub fn new(
        metadata: BallotMetaData,
        choice: Option<Choice>,
        validation: BallotValidation,
    ) -> Self {
        Self {
            metadata,
            choice,
            validation,
            _padding: Vec::new(),
        }
    }
    pub fn resize(&mut self) -> io::Result<()> {
        let bytes = serde_json::to_vec(&self)?;

        // Set padding such that Ballot will match predefined size
        let padding_size = if BALLOT_SIZE >= bytes.len() {
            BALLOT_SIZE - bytes.len()
        } else {
            return Err(Error::new(
                ErrorKind::Other,
                format!(
                    "Could not set padding for ballot because it exceeds the allowed size. Max size: {BALLOT_SIZE}B, Found size: {}",
                    bytes.len()
                ),
            ));
        };

        self._padding.resize(padding_size, 0);
        // Randomize to avoid determenistic compression
        // Compression may still occur, but it will not be possible to tell the original size
        rand::rng().fill(&mut self._padding[..]);

        Ok(())
    }

    pub fn get_metadata(&self) -> &BallotMetaData {
        &self.metadata
    }

    pub fn get_choice(&self) -> &Option<Choice> {
        &self.choice
    }

    pub fn get_validation(&self) -> &BallotValidation {
        &self.validation
    }
}
