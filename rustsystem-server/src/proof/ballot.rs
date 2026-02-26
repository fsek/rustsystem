use std::collections::HashSet;

use crate::proof::{Sha256ValidationInfo, ValidationInfo};

use rustsystem_core::APIError;
use serde::{Deserialize, Serialize};
use zkryptium::schemes::{algorithms::BbsBls12381Sha256, generics::BlindSignature};

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
    pub fn get_signature(&self) -> &BlindSignature<BbsBls12381Sha256> {
        &self.signature
    }
}
impl TryFrom<BallotValidation> for Sha256ValidationInfo {
    type Error = APIError;

    fn try_from(value: BallotValidation) -> Result<Self, Self::Error> {
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

#[derive(Serialize, Deserialize)]
pub struct Ballot {
    metadata: BallotMetaData,
    choice: Option<Choice>, // None for blank vote
    validation: BallotValidation,
    _padding: Vec<u8>,
}
impl Ballot {
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
