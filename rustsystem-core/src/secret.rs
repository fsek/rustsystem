//! Random secrets (session tokens, invite secrets, trustauth tickets) and how they are stored.
//!
//! Every secret is 32 random bytes, sent as base64url without padding. Services never store the
//! secret itself, only its SHA-256 ([`TokenHash`]), so a memory dump or log line can't be replayed
//! as a login.

use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use rand::RngCore;
use sha2::{Digest, Sha256};

pub type TokenHash = [u8; 32];

/// A fresh 32-byte secret, base64url-encoded.
pub fn new_token() -> String {
    let mut bytes = [0u8; 32];
    rand::rng().fill_bytes(&mut bytes);
    b64_encode(bytes)
}

/// What a service stores in place of a token.
pub fn hash_token(token: &str) -> TokenHash {
    Sha256::digest(token.as_bytes()).into()
}

pub fn sha256(bytes: &[u8]) -> [u8; 32] {
    Sha256::digest(bytes).into()
}

pub fn b64_encode(bytes: impl AsRef<[u8]>) -> String {
    URL_SAFE_NO_PAD.encode(bytes)
}

pub fn b64_decode(text: &str) -> Option<Vec<u8>> {
    URL_SAFE_NO_PAD.decode(text).ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tokens_are_unique_and_32_bytes() {
        let (a, b) = (new_token(), new_token());
        assert_ne!(a, b);
        assert_eq!(b64_decode(&a).unwrap().len(), 32);
    }

    #[test]
    fn hash_is_stable() {
        let t = new_token();
        assert_eq!(hash_token(&t), hash_token(&t));
        assert_ne!(hash_token(&t), hash_token(&new_token()));
    }
}
