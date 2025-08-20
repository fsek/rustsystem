use std::{
    error::{self, Error},
    fmt::{Debug, Display},
    time::Duration,
};

use bincode::{Decode, Encode};
use blake3::{Hash, Hasher};
use bls12_381_plus::elliptic_curve::hash2curve::ExpandMsg;
use getrandom;
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use serde_json::Value;
use wasm_bindgen::prelude::wasm_bindgen;
use zkryptium::{
    bbsplus::{ciphersuites::BbsCiphersuite, commitment::BlindFactor, keys::BBSplusPublicKey},
    keys::pair::KeyPair,
    schemes::{
        algorithms::{BBSplus, BbsBls12381Sha256, Scheme},
        generics::{BlindSignature, Commitment},
    },
};

mod ballot;
pub use ballot::*;

const TOKEN_SIZE: usize = 256;

#[derive(Serialize, Deserialize, Debug)]
pub enum RegistrationReject {
    SignatureFailure,
    AlreadyRegistered,

    MUIDNotFound,
    VoteInactive,

    Empty,
}
impl Display for RegistrationReject {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Registration failed due to {self:?}")
    }
}
impl error::Error for RegistrationReject {}

#[derive(Serialize, Deserialize)]
pub struct RegistrationSuccessResponse {
    signature: BlindSignature<BbsBls12381Sha256>,
    metadata: BallotMetaData,
}
impl RegistrationSuccessResponse {
    pub fn new(signature: BlindSignature<BbsBls12381Sha256>, metadata: BallotMetaData) -> Self {
        Self {
            signature,
            metadata,
        }
    }
}

#[wasm_bindgen]
pub struct WASMRegistrationResponse {
    rejected: Option<RegistrationReject>,
    accepted: Option<RegistrationSuccessResponse>,
}
impl WASMRegistrationResponse {
    pub fn new() -> Self {
        Self {
            rejected: None,
            accepted: None,
        }
    }
    pub fn into_response(self) -> Result<RegistrationSuccessResponse, RegistrationReject> {
        if let Some(rejected) = self.rejected {
            Err(rejected)
        } else if let Some(res) = self.accepted {
            Ok(res)
        } else {
            Err(RegistrationReject::Empty)
        }
    }

    pub fn signature(&self) -> Option<BlindSignature<BbsBls12381Sha256>> {
        Some(self.accepted.as_ref()?.signature.clone())
    }

    pub fn metadata(&self) -> Option<BallotMetaData> {
        Some(self.accepted.as_ref()?.metadata.clone())
    }

    pub fn is_valid(&self) -> bool {
        if let Some(_) = self.rejected {
            true
        } else if let Some(_) = self.accepted {
            true
        } else {
            false
        }
    }
    pub fn is_successful(&self) -> bool {
        if let Some(_) = self.accepted {
            true
        } else {
            false
        }
    }
}
impl From<RegistrationSuccessResponse> for WASMRegistrationResponse {
    fn from(value: RegistrationSuccessResponse) -> Self {
        Self {
            accepted: Some(value),
            rejected: None,
        }
    }
}
impl From<RegistrationReject> for WASMRegistrationResponse {
    fn from(value: RegistrationReject) -> Self {
        Self {
            rejected: Some(value),
            accepted: None,
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
#[wasm_bindgen]
pub enum ValidationReject {
    InvalidMetaData,
    MUIDNotFound,
    VotingInactive,

    SignatureInvalid,
    SignatureExpired,
}

#[derive(Serialize, Deserialize, Encode, Decode, Debug)]
pub struct ProofContext {
    voter_id: Vec<u8>,               // Voter's UUID
    meeting_id: Vec<u8>,             // Meeting's MUID
    registration_timestamp: Vec<u8>, // Timestamp (from UNIX EPOCH as seconds)
    checksum: Vec<u8>,
}
impl ProofContext {
    pub fn new(voter_id: Vec<u8>, meeting_id: Vec<u8>) -> Self {
        let registration_timestamp = Duration::from_millis(js_sys::Date::now() as u64)
            .as_secs()
            .to_be_bytes()
            .to_vec();
        let checksum = Self::calculate_checksum(&voter_id, &meeting_id, &registration_timestamp)
            .as_bytes()
            .to_vec();

        Self {
            voter_id,
            meeting_id,
            registration_timestamp,
            checksum,
        }
    }

    pub fn as_messages(&self) -> [Vec<u8>; 3] {
        [
            self.voter_id.clone(),
            self.meeting_id.clone(),
            self.registration_timestamp.clone(),
        ]
    }

    pub fn validate(&self) -> bool {
        let hash = Hash::from_slice(self.checksum.as_slice()).unwrap();

        hash == Self::calculate_checksum(
            &self.voter_id,
            &self.meeting_id,
            &self.registration_timestamp,
        )
    }

    fn calculate_checksum(voter_id: &Vec<u8>, meeting_id: &Vec<u8>, timestamp: &Vec<u8>) -> Hash {
        let mut hasher = Hasher::new();
        hasher.update(&voter_id);
        hasher.update(&meeting_id);
        hasher.update(&timestamp);
        hasher.finalize()
    }
}

pub trait ValidationInfo<S: Scheme>:
    Serialize + DeserializeOwned + Debug + From<BallotValidation>
where
    S::Ciphersuite: BbsCiphersuite,
    <S::Ciphersuite as BbsCiphersuite>::Expander: for<'a> ExpandMsg<'a>,
{
    fn new(
        proof: Vec<u8>,
        token: Vec<u8>,
        signature: BlindSignature<BBSplus<S::Ciphersuite>>,
    ) -> Self;
    fn get_proof(&self) -> BlindFactor;
}
#[derive(Serialize, Deserialize, Debug)]
pub struct Sha256ValidationInfo {
    // BlindFactor doesn't derive Serialize/Deserialize, so it is sent as a slice instead
    proof: [u8; 32],
    pub token: Vec<u8>,
    pub signature: BlindSignature<BbsBls12381Sha256>,
}
impl ValidationInfo<BbsBls12381Sha256> for Sha256ValidationInfo {
    fn new(proof: Vec<u8>, token: Vec<u8>, signature: BlindSignature<BbsBls12381Sha256>) -> Self {
        Sha256ValidationInfo {
            proof: proof.try_into().unwrap(),
            token,
            signature,
        }
    }
    fn get_proof(&self) -> BlindFactor {
        BlindFactor::from_bytes(&self.proof).unwrap()
    }
}

pub trait RegistrationInfo<S: Scheme>: Serialize + DeserializeOwned + Debug
where
    S::Ciphersuite: BbsCiphersuite,
    <S::Ciphersuite as BbsCiphersuite>::Expander: for<'a> ExpandMsg<'a>,
{
    fn new(context: ProofContext, commitment: Commitment<S>) -> Self;
}
#[derive(Serialize, Deserialize, Debug)]
pub struct Sha256RegistrationInfo {
    pub context: ProofContext,
    pub commitment: Commitment<BbsBls12381Sha256>,
}
impl RegistrationInfo<BbsBls12381Sha256> for Sha256RegistrationInfo {
    fn new(context: ProofContext, commitment: Commitment<BbsBls12381Sha256>) -> Self {
        Self {
            context,
            commitment,
        }
    }
}

pub trait Provider<S: Scheme>
where
    S::Ciphersuite: BbsCiphersuite,
    <S::Ciphersuite as BbsCiphersuite>::Expander: for<'a> ExpandMsg<'a>,
{
    type V: ValidationInfo<S>;
    type R: RegistrationInfo<S>;

    fn new_val_info(
        proof: Vec<u8>,
        token: Vec<u8>,
        signature: BlindSignature<BBSplus<S::Ciphersuite>>,
    ) -> Self::V {
        Self::V::new(proof, token, signature)
    }
    fn val_info_from_json(json: Value) -> Result<Self::V, Box<dyn Error>> {
        Ok(Self::V::deserialize(json)?)
    }

    fn new_reg_info(context: ProofContext, commitment: Commitment<S>) -> Self::R {
        Self::R::new(context, commitment)
    }
    fn reg_info_from_json(json: Value) -> Result<Self::R, Box<dyn Error>> {
        Ok(Self::R::deserialize(json)?)
    }

    fn validate_token(
        proof: BlindFactor,
        header: Vec<u8>,
        commited_token: Vec<u8>,
        authetication_pk: BBSplusPublicKey,
        signature: BlindSignature<BBSplus<S::Ciphersuite>>,
    ) -> Result<(), Box<dyn Error>> {
        Ok(signature.verify_blind_sign(
            &authetication_pk,
            Some(&header),
            None,
            Some(&[commited_token]),
            Some(&proof),
        )?)
    }
    fn sign_token(
        commitment: Commitment<BBSplus<S::Ciphersuite>>,
        header: Vec<u8>,
        keypair: KeyPair<BBSplus<S::Ciphersuite>>,
    ) -> Result<BlindSignature<BBSplus<S::Ciphersuite>>, Box<dyn Error>> {
        Ok(BlindSignature::<BBSplus<S::Ciphersuite>>::blind_sign(
            keypair.private_key(),
            keypair.public_key(),
            Some(&commitment.to_bytes()),
            Some(&header),
            None,
        )?)
    }

    fn generate_authentication_keys() -> KeyPair<BBSplus<S::Ciphersuite>> {
        let material: Vec<u8> = (0..S::Ciphersuite::IKM_LEN)
            .map(|_| {
                let mut buf = [0u8];
                getrandom::fill(&mut buf).unwrap();
                buf[0]
            })
            .collect();
        KeyPair::<BBSplus<S::Ciphersuite>>::generate(&material, None, None).unwrap()
    }

    fn generate_token(
        voter_id: Vec<u8>,
        meeting_id: Vec<u8>,
    ) -> Result<
        (
            ProofContext,
            Vec<u8>,
            Commitment<BBSplus<S::Ciphersuite>>,
            BlindFactor,
        ),
        Box<dyn Error>,
    > {
        let mut commited_token = vec![0u8; TOKEN_SIZE];
        getrandom::fill(&mut commited_token).unwrap();

        let (commitment, proof) =
            Commitment::<BBSplus<S::Ciphersuite>>::commit(Some(&[commited_token.clone()]))?;

        let context = ProofContext::new(voter_id, meeting_id);

        Ok((context, commited_token, commitment, proof))
    }
}

pub struct Sha256Provider;
impl Provider<BbsBls12381Sha256> for Sha256Provider {
    type V = Sha256ValidationInfo;
    type R = Sha256RegistrationInfo;
}
