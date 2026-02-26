use base64::{Engine, engine::general_purpose::STANDARD};
use chacha20poly1305::{
    ChaCha20Poly1305, Key, Nonce,
    aead::{Aead, KeyInit},
};
use hkdf::Hkdf;
use rand_core::OsRng;
use rustsystem_core::{APIError, APIErrorCode};
use sha2::Sha256;
use x25519_dalek::{EphemeralSecret, PublicKey};

use crate::{MUuid, vote_auth::Tally};

/// Parse a 32-byte X25519 public key from a SubjectPublicKeyInfo PEM file.
///
/// X25519 SPKI DER is always 44 bytes:
///   30 2a  30 05  06 03 2b 65 6e  03 21 00  <32-byte key>
/// The key is simply the last 32 bytes.
fn parse_x25519_spki_pem(pem: &str) -> Option<[u8; 32]> {
    let b64: String = pem.lines().filter(|l| !l.starts_with("-----")).collect();
    let der = STANDARD.decode(b64.trim()).ok()?;
    if der.len() < 32 {
        return None;
    }
    der[der.len() - 32..].try_into().ok()
}

/// Encrypt `plaintext` for an X25519 recipient public key using ECIES:
///   ephemeral X25519  →  ECDH shared secret  →  HKDF-SHA256  →  ChaCha20-Poly1305
///
/// Output layout: ephemeral_pk (32) ‖ nonce (12) ‖ ciphertext+tag
fn encrypt_for_x25519(recipient_pub: [u8; 32], plaintext: &[u8]) -> Result<Vec<u8>, APIError> {
    let ephemeral_sk = EphemeralSecret::random_from_rng(OsRng);
    let ephemeral_pk = PublicKey::from(&ephemeral_sk);
    let recipient_pk = PublicKey::from(recipient_pub);

    let shared_secret = ephemeral_sk.diffie_hellman(&recipient_pk);

    // Derive key (32 bytes) + nonce (12 bytes) from the shared secret via HKDF.
    // Use the ephemeral public key as additional context (salt) so that each
    // message produces independent key material even for the same recipient.
    let hk = Hkdf::<Sha256>::new(Some(ephemeral_pk.as_bytes()), shared_secret.as_bytes());
    let mut okm = [0u8; 44];
    hk.expand(b"rustsystem-tally-v1", &mut okm)
        .map_err(|_| APIError::from_error_code(APIErrorCode::CryptoError))?;

    // This is not ideal, but chacha20poly1305 depends on an older version of aead which depends on
    // an older version of generic-array...
    #[allow(deprecated)]
    let key = Key::from_slice(&okm[..32]);
    #[allow(deprecated)]
    let nonce = Nonce::from_slice(&okm[32..44]);

    let ciphertext = ChaCha20Poly1305::new(key)
        .encrypt(nonce, plaintext)
        .map_err(|_| APIError::from_error_code(APIErrorCode::CryptoError))?;

    let mut out = Vec::with_capacity(32 + 12 + ciphertext.len());
    out.extend_from_slice(ephemeral_pk.as_bytes());
    out.extend_from_slice(nonce);
    out.extend_from_slice(&ciphertext);
    Ok(out)
}

/// Serialize `tally` as JSON, encrypt it for the meeting's X25519 public key,
/// and write the result to `meetings/<muuid>/tally.enc`.
///
/// File format: ephemeral_pk (32) ‖ nonce (12) ‖ ChaCha20-Poly1305 ciphertext+tag
/// Plaintext:   UTF-8 JSON of the Tally struct
pub fn save_encrypted_tally(
    muuid: &MUuid,
    tally: &Tally,
    voters: Vec<String>,
) -> Result<(), APIError> {
    let meeting_dir = format!("meetings/{muuid}");
    let pub_key_path = format!("{meeting_dir}/pub_key.pem");
    let out_path = format!("{meeting_dir}/tally-{}.enc", chrono::offset::Local::now());

    let pem = std::fs::read_to_string(&pub_key_path)
        .map_err(|_| APIError::from_error_code(APIErrorCode::IoError))?;
    let pub_key = parse_x25519_spki_pem(&pem)
        .ok_or_else(|| APIError::new(APIErrorCode::IoError, "Failed to parse X25519 public key from SPKI PEM", 500))?;

    let plaintext = serde_json::to_vec(&(tally, voters))
        .map_err(|_| APIError::from_error_code(APIErrorCode::IoError))?;
    let encrypted = encrypt_for_x25519(pub_key, &plaintext)?;

    std::fs::write(&out_path, &encrypted)
        .map_err(|_| APIError::from_error_code(APIErrorCode::IoError))
}
