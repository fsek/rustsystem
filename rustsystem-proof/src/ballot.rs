use std::{
    collections::HashMap,
    io::{self, Error, ErrorKind},
};

use serde::{Deserialize, Serialize};
use zkryptium::schemes::{algorithms::BbsBls12381Sha256, generics::BlindSignature};

use crate::{Sha256ValidationInfo, ValidationInfo};

pub type VoteRoundID = u128;
pub type CandidateID = u8;
pub type ProtocolVersion = u8;

#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Eq)]
pub enum VoteMethod {
    Dichotomous,
    Plurality,
    RankedChoice,
    Approval,
    Score,
    STAR,
}

#[derive(Serialize, Deserialize)]
pub enum Choice {
    // true for "Yes", false for "No"
    Dichotomous(bool),

    // Aka: First Past the Post (FPTP). Contains the ID of the chosen candidate.
    Plurality(CandidateID),

    // Aka: Instant Runoff. Contains (in order) the IDs of the candidates.
    RankedChoice(Vec<CandidateID>),

    // Contains the IDs of candidates approved by voter.
    Approval(Vec<CandidateID>),

    // Contains the IDs of the candidates alongside their respective scores.
    Score(HashMap<CandidateID, u8>),

    // Contains the IDs of the candidates alongside their respective scores.
    STAR(HashMap<CandidateID, u8>),
}

#[derive(Serialize, Deserialize, Clone)]
// All fields are private since they should not change once set
pub struct BallotValidation {
    proof: Vec<u8>,
    token: Vec<u8>,
    signature: BlindSignature<BbsBls12381Sha256>,
}
// Getter functions for private fields
impl BallotValidation {
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

#[derive(Serialize, Deserialize, PartialEq, Eq, Clone, Copy)]
// All fields are private since they should not change once set
pub struct BallotMetaData {
    method: VoteMethod,
    protocol_version: ProtocolVersion,
}

impl BallotMetaData {
    pub fn new(method: VoteMethod, protocol_version: ProtocolVersion) -> Self {
        Self {
            method,
            protocol_version,
        }
    }
    // Getter functions for private fields
    pub fn get_method(&self) -> VoteMethod {
        self.method
    }
    pub fn get_protocol_version(&self) -> ProtocolVersion {
        self.protocol_version
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
    pub fn resize(&mut self) -> io::Result<()> {
        let bytes = serde_json::to_vec(&self)?;

        // Set padding such that Ballot will match predefined size
        let padding_size = BALLOT_SIZE - bytes.len();
        if padding_size < 0 {
            return Err(Error::new(
                ErrorKind::Other,
                format!(
                    "Could not set padding for ballot because it exceeds the allowed size. Max size: {BALLOT_SIZE}B, Found size: {}",
                    bytes.len()
                ),
            ));
        } else {
            self._padding.resize(padding_size, 0);
            // Randomize to avoid determenistic compression
            // Compression may still occur, but it will not be possible to tell the original size
            getrandom::fill(&mut self._padding);
        }
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
