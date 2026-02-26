use bls12_381_plus::elliptic_curve::hash2curve::ExpandMsg;
use rustsystem_core::{APIError, APIErrorCode};
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use std::fmt::Debug;
use zkryptium::{
    bbsplus::{ciphersuites::BbsCiphersuite, commitment::BlindFactor, keys::BBSplusPublicKey},
    schemes::{
        algorithms::{BBSplus, BbsBls12381Sha256, Scheme},
        generics::BlindSignature,
    },
};

mod ballot;
pub use ballot::*;

pub trait ValidationInfo<S: Scheme>:
    Serialize + DeserializeOwned + Debug + TryFrom<BallotValidation, Error = APIError>
where
    S::Ciphersuite: BbsCiphersuite,
    <S::Ciphersuite as BbsCiphersuite>::Expander: for<'a> ExpandMsg<'a>,
{
    fn new(
        proof: Vec<u8>,
        token: Vec<u8>,
        signature: BlindSignature<BBSplus<S::Ciphersuite>>,
    ) -> Result<Self, APIError>;
    fn get_proof(&self) -> Result<BlindFactor, APIError>;
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Sha256ValidationInfo {
    // BlindFactor doesn't derive Serialize/Deserialize, so it is sent as a slice instead
    proof: [u8; 32],
    pub token: Vec<u8>,
    pub signature: BlindSignature<BbsBls12381Sha256>,
}

impl ValidationInfo<BbsBls12381Sha256> for Sha256ValidationInfo {
    fn new(proof: Vec<u8>, token: Vec<u8>, signature: BlindSignature<BbsBls12381Sha256>) -> Result<Self, APIError> {
        let proof: [u8; 32] = proof
            .try_into()
            .map_err(|_| APIError::new(APIErrorCode::SignatureInvalid, "Proof must be exactly 32 bytes", 401))?;
        Ok(Sha256ValidationInfo { proof, token, signature })
    }

    fn get_proof(&self) -> Result<BlindFactor, APIError> {
        BlindFactor::from_bytes(&self.proof)
            .map_err(|_| APIError::from_error_code(APIErrorCode::SignatureInvalid))
    }
}

pub trait Provider<S: Scheme>
where
    S::Ciphersuite: BbsCiphersuite,
    <S::Ciphersuite as BbsCiphersuite>::Expander: for<'a> ExpandMsg<'a>,
{
    type V: ValidationInfo<S>;

    fn validate_token(
        proof: BlindFactor,
        header: Vec<u8>,
        commited_token: Vec<u8>,
        authetication_pk: BBSplusPublicKey,
        signature: BlindSignature<BBSplus<S::Ciphersuite>>,
    ) -> Result<(), APIError> {
        signature
            .verify_blind_sign(
                &authetication_pk,
                Some(&header),
                None,
                Some(&[commited_token]),
                Some(&proof),
            )
            .map_err(|_| APIError::from_error_code(APIErrorCode::SignatureInvalid))
    }
}

pub struct Sha256Provider;
impl Provider<BbsBls12381Sha256> for Sha256Provider {
    type V = Sha256ValidationInfo;
}
