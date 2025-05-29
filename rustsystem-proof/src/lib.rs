use std::{
    error::Error,
    fmt::Debug,
    marker::PhantomData,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use bincode::{Decode, Encode};
use blake3::{Hash, Hasher};
use bls12_381_plus::elliptic_curve::hash2curve::ExpandMsg;
use getrandom;
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use serde_json::Value;
use zkryptium::{
    bbsplus::{ciphersuites::BbsCiphersuite, commitment::BlindFactor, keys::BBSplusPublicKey},
    keys::pair::KeyPair,
    schemes::{
        algorithms::{BBSplus, BbsBls12381Sha256, Scheme},
        generics::{BlindSignature, Commitment},
    },
};

const TOKEN_SIZE: usize = 256;

#[derive(Serialize, Deserialize, Debug)]
pub enum RegistrationRejectReason {
    SignatureFailure,
    AlreadyRegistered,
}

#[derive(Serialize, Deserialize, Debug)]
pub enum RegistrationResponse {
    Rejected(RegistrationRejectReason),
    Accepted(BlindSignature<BbsBls12381Sha256>),
}

#[derive(Serialize, Deserialize, Debug)]
pub enum ValidationRejectReason {
    SignatureInvalid,
    SignatureExpired,
}

#[derive(Serialize, Deserialize, Debug)]
pub enum ValidationResponse {
    Rejected(ValidationRejectReason),
    Accepted,
}

#[derive(Serialize, Deserialize, Encode, Decode, Debug)]
pub struct ProofContext {
    voter_id: Vec<u8>,               // Hash of voter's id
    round_hash: Vec<u8>,             // Hash of voting round
    registration_timestamp: Vec<u8>, // Timestamp (from UNIX EPOCH as seconds)
    checksum: Vec<u8>,
}
impl ProofContext {
    pub fn new(voter_id: Vec<u8>, round_hash: Vec<u8>) -> Self {
        let registration_timestamp = Duration::from_millis(js_sys::Date::now() as u64)
            .as_secs()
            .to_be_bytes()
            .to_vec();
        let checksum = Self::calculate_checksum(&voter_id, &round_hash, &registration_timestamp)
            .as_bytes()
            .to_vec();

        Self {
            voter_id,
            round_hash,
            registration_timestamp,
            checksum,
        }
    }

    pub fn as_messages(&self) -> [Vec<u8>; 3] {
        [
            self.voter_id.clone(),
            self.round_hash.clone(),
            self.registration_timestamp.clone(),
        ]
    }

    pub fn validate(&self) -> bool {
        let hash = Hash::from_slice(self.checksum.as_slice()).unwrap();

        hash == Self::calculate_checksum(
            &self.voter_id,
            &self.round_hash,
            &self.registration_timestamp,
        )
    }

    fn calculate_checksum(voter_id: &Vec<u8>, voting_round: &Vec<u8>, timestamp: &Vec<u8>) -> Hash {
        let mut hasher = Hasher::new();
        hasher.update(&voter_id);
        hasher.update(&voting_round);
        hasher.update(&timestamp);
        hasher.finalize()
    }
}

pub trait ValidationInfo<S: Scheme>: Serialize + DeserializeOwned + Debug
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
                getrandom::getrandom(&mut buf).unwrap();
                buf[0]
            })
            .collect();
        KeyPair::<BBSplus<S::Ciphersuite>>::generate(&material, None, None).unwrap()
    }

    fn generate_token(
        voter_id: Vec<u8>,
        round_hash: Vec<u8>,
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
        getrandom::getrandom(&mut commited_token).unwrap();

        let (commitment, proof) =
            Commitment::<BBSplus<S::Ciphersuite>>::commit(Some(&[commited_token.clone()]))?;

        let context = ProofContext::new(voter_id, round_hash);

        Ok((context, commited_token, commitment, proof))
    }
}

pub struct Sha256Provider;
impl Provider<BbsBls12381Sha256> for Sha256Provider {
    type V = Sha256ValidationInfo;
    type R = Sha256RegistrationInfo;
}
