use base64::{Engine, engine::general_purpose::STANDARD};
use chacha20poly1305::{
    ChaCha20Poly1305, Key, Nonce,
    aead::{Aead, KeyInit},
};
use hkdf::Hkdf;
use sha2::Sha256;
use x25519_dalek::{PublicKey, StaticSecret};

/// Parse a 32-byte X25519 private key from a PKCS#8 PEM file.
///
/// X25519 PKCS#8 DER is always 48 bytes:
///   30 2e  02 01 00  30 05  06 03 2b 65 6e  04 22  04 20  <32-byte key>
/// The private scalar is simply the last 32 bytes.
fn parse_x25519_pkcs8_pem(pem: &str) -> Option<[u8; 32]> {
    let b64: String = pem.lines().filter(|l| !l.starts_with("-----")).collect();
    let der = STANDARD.decode(b64.trim()).ok()?;
    if der.len() < 32 {
        return None;
    }
    der[der.len() - 32..].try_into().ok()
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() != 3 {
        eprintln!("Usage: {} <tally.enc> <private_x25519.pem>", args[0]);
        std::process::exit(1);
    }

    let enc_path = &args[1];
    let key_path = &args[2];

    let encrypted = std::fs::read(enc_path).unwrap_or_else(|e| {
        eprintln!("Failed to read {enc_path}: {e}");
        std::process::exit(1);
    });

    // Minimum: 32 (ephemeral pk) + 12 (nonce) + 16 (Poly1305 tag) = 60 bytes
    if encrypted.len() < 60 {
        eprintln!(
            "File too short to be a valid tally.enc ({} bytes)",
            encrypted.len()
        );
        std::process::exit(1);
    }

    let pem = std::fs::read_to_string(key_path).unwrap_or_else(|e| {
        eprintln!("Failed to read {key_path}: {e}");
        std::process::exit(1);
    });

    let sk_bytes = parse_x25519_pkcs8_pem(&pem).unwrap_or_else(|| {
        eprintln!("Failed to parse X25519 private key from PKCS#8 PEM");
        std::process::exit(1);
    });

    // Split the file: ephemeral_pk (32) ‖ nonce (12) ‖ ciphertext+tag
    let ephemeral_pk_bytes: [u8; 32] = encrypted[..32].try_into().unwrap();
    #[allow(deprecated)]
    let nonce = Nonce::from_slice(&encrypted[32..44]);
    let ciphertext = &encrypted[44..];

    let static_sk = StaticSecret::from(sk_bytes);
    let ephemeral_pk = PublicKey::from(ephemeral_pk_bytes);
    let shared_secret = static_sk.diffie_hellman(&ephemeral_pk);

    // Replicate HKDF from encryption: salt = ephemeral_pk, info = "rustsystem-tally-v1"
    // Only 32 bytes needed for the key (first block of HKDF-Expand = same bytes as in
    // the 44-byte expansion used during encryption).
    let hk = Hkdf::<Sha256>::new(Some(ephemeral_pk.as_bytes()), shared_secret.as_bytes());
    let mut key_bytes = [0u8; 32];
    hk.expand(b"rustsystem-tally-v1", &mut key_bytes)
        .expect("HKDF-SHA256 expand: output length is valid");

    #[allow(deprecated)]
    let key = Key::from_slice(&key_bytes);

    let plaintext = ChaCha20Poly1305::new(key)
        .decrypt(nonce, ciphertext)
        .unwrap_or_else(|_| {
            eprintln!("Decryption failed — wrong private key or corrupted file");
            std::process::exit(1);
        });

    let text = String::from_utf8(plaintext).unwrap_or_else(|_| {
        eprintln!("Decrypted content is not valid UTF-8");
        std::process::exit(1);
    });

    println!("{text}");
}
