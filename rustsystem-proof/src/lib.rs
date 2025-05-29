use std::{
    error::Error,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use bincode::{Decode, Encode};
use blake3::{Hash, Hasher};
use bls12_381_plus::elliptic_curve::hash2curve::ExpandMsg;
use getrandom;
use serde::{Deserialize, Serialize};
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
pub struct RegistrationInfo<S: Scheme>
where
    S::Ciphersuite: BbsCiphersuite,
    <S::Ciphersuite as BbsCiphersuite>::Expander: for<'a> ExpandMsg<'a>,
{
    pub context: ProofContext,
    pub commitment: Commitment<BBSplus<S::Ciphersuite>>,
}
impl RegistrationInfo<BbsBls12381Sha256> {
    pub fn new(context: ProofContext, commitment: Commitment<BbsBls12381Sha256>) -> Self {
        Self {
            context,
            commitment,
        }
    }
}

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
pub struct ValidationInfo {
    // BlindFactor doesn't derive Serialize/Deserialize, so it is sent as a slice instead
    proof: [u8; 32],
    pub token: Vec<u8>,
    pub signature: BlindSignature<BbsBls12381Sha256>,
}
impl ValidationInfo {
    pub fn new(
        proof: Vec<u8>,
        token: Vec<u8>,
        signature: BlindSignature<BbsBls12381Sha256>,
    ) -> Self {
        ValidationInfo {
            proof: proof.try_into().unwrap(),
            token,
            signature,
        }
    }

    pub fn get_proof(&self) -> BlindFactor {
        BlindFactor::from_bytes(&self.proof).unwrap()
    }
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

    fn validate(&self) -> bool {
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

pub fn generate_token_sha(
    voter_id: Vec<u8>,
    round_hash: Vec<u8>,
) -> Result<
    (
        ProofContext,
        Vec<u8>,
        Commitment<BbsBls12381Sha256>,
        BlindFactor,
    ),
    Box<dyn Error>,
> {
    generate_token_generic::<BbsBls12381Sha256>(voter_id, round_hash)
}

fn generate_token_generic<S: Scheme>(
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
>
where
    S::Ciphersuite: BbsCiphersuite,
    <S::Ciphersuite as BbsCiphersuite>::Expander: for<'a> ExpandMsg<'a>,
{
    let mut commited_token = vec![0u8; TOKEN_SIZE];
    getrandom::getrandom(&mut commited_token).unwrap();

    let (commitment, proof) =
        Commitment::<BBSplus<S::Ciphersuite>>::commit(Some(&[commited_token.clone()]))?;

    let context = ProofContext::new(voter_id, round_hash);

    Ok((context, commited_token, commitment, proof))
}

pub fn generate_authentication_token_sha() -> KeyPair<BbsBls12381Sha256> {
    generate_authentication_token_generic::<BbsBls12381Sha256>()
}

fn generate_authentication_token_generic<S: Scheme>() -> KeyPair<BBSplus<S::Ciphersuite>>
where
    S::Ciphersuite: BbsCiphersuite,
    <S::Ciphersuite as BbsCiphersuite>::Expander: for<'a> ExpandMsg<'a>,
{
    let material: Vec<u8> = (0..S::Ciphersuite::IKM_LEN)
        .map(|_| {
            let mut buf = [0u8];
            getrandom::getrandom(&mut buf).unwrap();
            buf[0]
        })
        .collect();
    KeyPair::<BBSplus<S::Ciphersuite>>::generate(&material, None, None).unwrap()
}

pub fn authenticate_token_sha(
    commitment: Commitment<BbsBls12381Sha256>,
    header: Vec<u8>,
    keypair: KeyPair<BbsBls12381Sha256>,
) -> Result<BlindSignature<BbsBls12381Sha256>, Box<dyn Error>> {
    authenticate_token_generic::<BbsBls12381Sha256>(commitment, header, keypair)
}

fn authenticate_token_generic<S: Scheme>(
    commitment: Commitment<BBSplus<S::Ciphersuite>>,
    header: Vec<u8>,
    keypair: KeyPair<BBSplus<S::Ciphersuite>>,
) -> Result<BlindSignature<BBSplus<S::Ciphersuite>>, Box<dyn Error>>
where
    S::Ciphersuite: BbsCiphersuite,
    <S::Ciphersuite as BbsCiphersuite>::Expander: for<'a> ExpandMsg<'a>,
{
    Ok(BlindSignature::<BBSplus<S::Ciphersuite>>::blind_sign(
        keypair.private_key(),
        keypair.public_key(),
        Some(&commitment.to_bytes()),
        Some(&header),
        None,
    )?)
}

pub fn validate_token_sha(
    proof: BlindFactor,
    header: Vec<u8>,
    commited_token: Vec<u8>,
    authetication_pk: BBSplusPublicKey,
    signature: BlindSignature<BbsBls12381Sha256>,
) -> Result<(), Box<dyn Error>> {
    validate_token_generic::<BbsBls12381Sha256>(
        proof,
        header,
        commited_token,
        authetication_pk,
        signature,
    )
}

fn validate_token_generic<S: Scheme>(
    proof: BlindFactor,
    header: Vec<u8>,
    commited_token: Vec<u8>,
    authetication_pk: BBSplusPublicKey,
    signature: BlindSignature<BBSplus<S::Ciphersuite>>,
) -> Result<(), Box<dyn Error>>
where
    S::Ciphersuite: BbsCiphersuite,
    <S::Ciphersuite as BbsCiphersuite>::Expander: for<'a> ExpandMsg<'a>,
{
    Ok(signature.verify_blind_sign(
        &authetication_pk,
        Some(&header),
        None,
        Some(&[commited_token]),
        Some(&proof),
    )?)
}
