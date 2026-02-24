use std::{error::Error, fmt::Debug};

use bls12_381_plus::elliptic_curve::hash2curve::ExpandMsg;
use serde::{Deserialize, Serialize, de::DeserializeOwned};
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
    ) -> Result<(), Box<dyn Error>> {
        Ok(signature.verify_blind_sign(
            &authetication_pk,
            Some(&header),
            None,
            Some(&[commited_token]),
            Some(&proof),
        )?)
    }
}

pub struct Sha256Provider;
impl Provider<BbsBls12381Sha256> for Sha256Provider {
    type V = Sha256ValidationInfo;
}
