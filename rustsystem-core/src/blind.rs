//! RSA blind signatures (RFC 9474, `RSABSSA-SHA384-PSS-Randomized`) — the only cryptography
//! in the voting protocol. See `docs/PROTOCOL.md` §5.
//!
//! - Trustauth uses [`RoundKey::generate`] and [`RoundKey::blind_sign`].
//! - The server uses [`RoundPublicKey::verify`].
//! - The browser does the blinding with `@cloudflare/blindrsa-ts`; [`client`] is the same flow in
//!   Rust, used by tests.
//!
//! A *prepared* message is `randomizer (32 bytes) ‖ msg`, exactly what blindrsa-ts's `prepare()`
//! produces. The randomizer is part of what gets signed, so it travels with the ballot.

use blind_rsa_signatures::{
    BlindSignature, DefaultRng, KeyPairSha384PSSRandomized, MessageRandomizer,
    PublicKeySha384PSSRandomized, SecretKeySha384PSSRandomized, Signature,
};

pub const MODULUS_BITS: usize = 2048;
/// Length of a blinded message, a blind signature and a signature.
pub const SIGNATURE_LEN: usize = MODULUS_BITS / 8;
pub const RANDOMIZER_LEN: usize = 32;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BlindError {
    KeyGeneration,
    Encoding,
    /// The input is not something we would ever sign or accept (wrong length, etc).
    Malformed,
    InvalidSignature,
}

/// A round's key pair. Lives only in trustauth.
pub struct RoundKey {
    secret: SecretKeySha384PSSRandomized,
    public: RoundPublicKey,
}

impl std::fmt::Debug for RoundKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("RoundKey(<secret>)")
    }
}

impl RoundKey {
    /// Takes 25–100 ms; call from `spawn_blocking`.
    pub fn generate() -> Result<Self, BlindError> {
        let kp = KeyPairSha384PSSRandomized::generate(&mut DefaultRng, MODULUS_BITS)
            .map_err(|_| BlindError::KeyGeneration)?;
        Ok(Self {
            secret: kp.sk,
            public: RoundPublicKey(kp.pk),
        })
    }

    pub fn public_key(&self) -> &RoundPublicKey {
        &self.public
    }

    pub fn blind_sign(&self, blinded: &[u8]) -> Result<Vec<u8>, BlindError> {
        if blinded.len() != SIGNATURE_LEN {
            return Err(BlindError::Malformed);
        }
        self.secret
            .blind_sign(blinded)
            .map(|sig| sig.0)
            .map_err(|_| BlindError::Malformed)
    }
}

/// A round's public key. Everyone may have it.
#[derive(Clone)]
pub struct RoundPublicKey(PublicKeySha384PSSRandomized);

impl RoundPublicKey {
    /// SPKI DER with the `rsaEncryption` OID; the browser imports it as RSA-PSS / SHA-384.
    pub fn to_der(&self) -> Result<Vec<u8>, BlindError> {
        self.0.to_der().map_err(|_| BlindError::Encoding)
    }

    pub fn from_der(der: &[u8]) -> Result<Self, BlindError> {
        PublicKeySha384PSSRandomized::from_der(der)
            .map(Self)
            .map_err(|_| BlindError::Encoding)
    }

    /// Checks `sig` over `prepared = randomizer ‖ msg` and returns `msg`.
    pub fn verify<'a>(&self, prepared: &'a [u8], sig: &[u8]) -> Result<&'a [u8], BlindError> {
        if prepared.len() <= RANDOMIZER_LEN || sig.len() != SIGNATURE_LEN {
            return Err(BlindError::Malformed);
        }
        let (randomizer, msg) = prepared.split_at(RANDOMIZER_LEN);
        let randomizer = MessageRandomizer(randomizer.try_into().map_err(|_| BlindError::Malformed)?);
        self.0
            .verify(&Signature(sig.to_vec()), Some(randomizer), msg)
            .map(|()| msg)
            .map_err(|_| BlindError::InvalidSignature)
    }
}

/// The browser's half of the protocol, for tests. Production clients use blindrsa-ts.
pub mod client {
    use super::*;

    pub struct Blinded {
        /// Send this to trustauth.
        pub blinded: Vec<u8>,
        state: blind_rsa_signatures::BlindingResult,
        msg: Vec<u8>,
    }

    pub fn blind(pk: &RoundPublicKey, msg: &[u8]) -> Result<Blinded, BlindError> {
        let state = pk.0.blind(&mut DefaultRng, msg).map_err(|_| BlindError::Malformed)?;
        Ok(Blinded {
            blinded: state.blind_message.0.clone(),
            state,
            msg: msg.to_vec(),
        })
    }

    /// Returns `(prepared, sig)`, ready to submit as a ballot. Fails unless the signature verifies
    /// under `pk`, which is what stops trustauth from signing with a voter-specific key.
    pub fn finalize(
        pk: &RoundPublicKey,
        blinded: &Blinded,
        blind_sig: &[u8],
    ) -> Result<(Vec<u8>, Vec<u8>), BlindError> {
        let sig = pk
            .0
            .finalize(&BlindSignature(blind_sig.to_vec()), &blinded.state, &blinded.msg)
            .map_err(|_| BlindError::InvalidSignature)?;
        let randomizer = blinded.state.msg_randomizer.ok_or(BlindError::Malformed)?;
        let mut prepared = randomizer.0.to_vec();
        prepared.extend_from_slice(&blinded.msg);
        Ok((prepared, sig.0))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sign_round_trip(key: &RoundKey, msg: &[u8]) -> (Vec<u8>, Vec<u8>) {
        let blinded = client::blind(key.public_key(), msg).unwrap();
        let blind_sig = key.blind_sign(&blinded.blinded).unwrap();
        client::finalize(key.public_key(), &blinded, &blind_sig).unwrap()
    }

    #[test]
    fn round_trip_verifies_and_returns_message() {
        let key = RoundKey::generate().unwrap();
        let (prepared, sig) = sign_round_trip(&key, b"{\"v\":1}");
        assert_eq!(key.public_key().verify(&prepared, &sig).unwrap(), b"{\"v\":1}");
    }

    #[test]
    fn signer_never_sees_the_final_signature() {
        let key = RoundKey::generate().unwrap();
        let blinded = client::blind(key.public_key(), b"ballot").unwrap();
        let blind_sig = key.blind_sign(&blinded.blinded).unwrap();
        let (prepared, sig) = client::finalize(key.public_key(), &blinded, &blind_sig).unwrap();
        assert_ne!(blind_sig, sig);
        assert_ne!(blinded.blinded, sig);
        assert!(!prepared.windows(blinded.blinded.len()).any(|w| w == blinded.blinded));
    }

    #[test]
    fn tampering_is_rejected() {
        let key = RoundKey::generate().unwrap();
        let (mut prepared, sig) = sign_round_trip(&key, b"choice 0");
        *prepared.last_mut().unwrap() ^= 1;
        assert_eq!(
            key.public_key().verify(&prepared, &sig),
            Err(BlindError::InvalidSignature)
        );
    }

    #[test]
    fn other_rounds_key_is_rejected() {
        let (a, b) = (RoundKey::generate().unwrap(), RoundKey::generate().unwrap());
        let (prepared, sig) = sign_round_trip(&a, b"ballot");
        assert_eq!(b.public_key().verify(&prepared, &sig), Err(BlindError::InvalidSignature));
    }

    #[test]
    fn finalize_rejects_signature_from_another_key() {
        // Key tagging: trustauth signs with a key other than the one the server published.
        let (published, tagged) = (RoundKey::generate().unwrap(), RoundKey::generate().unwrap());
        for _ in 0..8 {
            let blinded = client::blind(published.public_key(), b"ballot").unwrap();
            // The blinded value may not fit under the other key's modulus; any answer is wrong.
            let blind_sig = tagged.blind_sign(&blinded.blinded).unwrap_or_else(|_| vec![7u8; SIGNATURE_LEN]);
            assert!(client::finalize(published.public_key(), &blinded, &blind_sig).is_err());
        }
    }

    #[test]
    fn malformed_inputs_are_rejected() {
        let key = RoundKey::generate().unwrap();
        assert_eq!(key.blind_sign(&[0u8; 10]), Err(BlindError::Malformed));
        assert_eq!(key.public_key().verify(&[0u8; 32], &[0u8; SIGNATURE_LEN]), Err(BlindError::Malformed));
        assert_eq!(key.public_key().verify(&[0u8; 40], &[0u8; 3]), Err(BlindError::Malformed));
    }

    #[test]
    fn public_key_der_round_trips() {
        let key = RoundKey::generate().unwrap();
        let der = key.public_key().to_der().unwrap();
        let (prepared, sig) = sign_round_trip(&key, b"x");
        let pk = RoundPublicKey::from_der(&der).unwrap();
        assert!(pk.verify(&prepared, &sig).is_ok());
    }
}
