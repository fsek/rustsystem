//! Decrypts a Rustsystem tally file offline (`docs/PROTOCOL.md` §8).
//!
//! ```text
//! decrypt-tally meetings/<meeting id>/tally-20270314T190211Z-3f2c1a9b.enc
//! Meeting password: ********
//! { "meeting": "Vårmöte 2027", "round": "Ordförande", ... }
//! ```
//!
//! The file's header carries the Argon2id salt and cost, so the password is all that's
//! needed. Set `RUSTSYSTEM_TALLY_PASSWORD` to skip the prompt (e.g. in scripts).

use std::process::ExitCode;

use argon2::{Algorithm, Argon2, Params, Version};
use chacha20poly1305::{
    ChaCha20Poly1305, Key, Nonce,
    aead::{Aead, KeyInit, Payload},
};
use hkdf::Hkdf;
use sha2::Sha256;
use x25519_dalek::x25519;

const MAGIC: &[u8; 5] = b"RSTLY";
const VERSION: u8 = 2;
const KDF_ARGON2ID: u8 = 1;
const HEADER_LEN: usize = 32;
const HKDF_INFO: &[u8] = b"rustsystem-tally-v2";
/// Header, ephemeral key, nonce, and the Poly1305 tag.
const MIN_LEN: usize = HEADER_LEN + 32 + 12 + 16;

#[derive(Debug, PartialEq, Eq)]
struct Kdf {
    salt: [u8; 16],
    t_cost: u32,
    m_cost_kib: u32,
    p_cost: u8,
}

fn parse_header(file: &[u8]) -> Result<Kdf, String> {
    if file.len() < MIN_LEN {
        return Err(format!("The file is too short to be a tally file ({} bytes).", file.len()));
    }
    if &file[..5] != MAGIC {
        return Err("This is not a Rustsystem tally file.".into());
    }
    if file[5] != VERSION {
        return Err(format!("Unsupported tally file version {} (this tool reads version {VERSION}).", file[5]));
    }
    if file[6] != KDF_ARGON2ID {
        return Err(format!("Unknown key-derivation id {}.", file[6]));
    }
    Ok(Kdf {
        salt: file[7..23].try_into().unwrap(),
        t_cost: u32::from_be_bytes(file[23..27].try_into().unwrap()),
        m_cost_kib: u32::from_be_bytes(file[27..31].try_into().unwrap()),
        p_cost: file[31],
    })
}

/// The meeting's X25519 private key, exactly as the browser derived it at meeting creation.
fn derive_private_key(password: &str, kdf: &Kdf) -> Result<[u8; 32], String> {
    let params = Params::new(kdf.m_cost_kib, kdf.t_cost, kdf.p_cost.into(), Some(32))
        .map_err(|e| format!("Invalid key-derivation parameters in the file: {e}"))?;
    let mut key = [0u8; 32];
    Argon2::new(Algorithm::Argon2id, Version::V0x13, params)
        .hash_password_into(password.as_bytes(), &kdf.salt, &mut key)
        .map_err(|e| format!("Key derivation failed: {e}"))?;
    Ok(key)
}

fn decrypt(file: &[u8], private_key: [u8; 32]) -> Result<Vec<u8>, String> {
    let header = &file[..HEADER_LEN];
    let ephemeral: [u8; 32] = file[HEADER_LEN..HEADER_LEN + 32].try_into().unwrap();
    let nonce = &file[HEADER_LEN + 32..HEADER_LEN + 44];
    let shared = x25519(private_key, ephemeral);

    let mut okm = [0u8; 44];
    Hkdf::<Sha256>::new(Some(&ephemeral), &shared)
        .expand(HKDF_INFO, &mut okm)
        .map_err(|_| "HKDF failed".to_string())?;
    // chacha20poly1305 0.10 still uses generic-array 0.14's deprecated constructors.
    #[allow(deprecated)]
    let (key, nonce) = (Key::from_slice(&okm[..32]), Nonce::from_slice(nonce));
    ChaCha20Poly1305::new(key)
        .decrypt(nonce, Payload { msg: &file[HEADER_LEN + 44..], aad: header })
        .map_err(|_| "Decryption failed: wrong password, or the file is damaged.".into())
}

fn run() -> Result<String, String> {
    let path = std::env::args().nth(1).ok_or("Usage: decrypt-tally <tally-file.enc>")?;
    let file = std::fs::read(&path).map_err(|e| format!("Can't read {path}: {e}"))?;
    let kdf = parse_header(&file)?;
    let password = match std::env::var("RUSTSYSTEM_TALLY_PASSWORD") {
        Ok(p) => p,
        Err(_) => rpassword::prompt_password("Meeting password: ").map_err(|e| format!("Can't read password: {e}"))?,
    };
    let plaintext = decrypt(&file, derive_private_key(&password, &kdf)?)?;
    String::from_utf8(plaintext).map_err(|_| "The decrypted tally is not valid UTF-8.".into())
}

fn main() -> ExitCode {
    match run() {
        Ok(text) => {
            println!("{text}");
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("{e}");
            ExitCode::FAILURE
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Shared with `frontend/src/utils/cryptoGen.test.ts`: the browser must derive exactly
    /// this key from this password and salt, or tally files can't be decrypted offline.
    #[test]
    fn kdf_test_vector() {
        let kdf = Kdf { salt: [7; 16], t_cost: 3, m_cost_kib: 65536, p_cost: 1 };
        let key = derive_private_key("correct horse battery staple", &kdf).unwrap();
        assert_eq!(hex(&key), "6ad10af97f1744119bd7135c85121dc589794f9c5d646200b8ad4d6becf15084");

        let salt: [u8; 16] = [0x00, 0x11, 0x22, 0x33, 0x44, 0x55, 0x66, 0x77, 0x88, 0x99, 0xaa, 0xbb, 0xcc, 0xdd, 0xee, 0xff];
        let kdf = Kdf { salt, t_cost: 3, m_cost_kib: 65536, p_cost: 1 };
        let key = derive_private_key("correct horse battery staple", &kdf).unwrap();
        assert_eq!(hex(&key), "c63a7e80f29a251ff0f1067c51d08ff12594199c5d2bd4a51d95348f3a205883");
    }

    #[test]
    fn rejects_other_files() {
        assert!(parse_header(b"short").is_err());
        let mut f = vec![0u8; MIN_LEN];
        assert!(parse_header(&f).unwrap_err().contains("not a Rustsystem"));
        f[..5].copy_from_slice(MAGIC);
        f[5] = 1;
        assert!(parse_header(&f).unwrap_err().contains("version 1"));
    }

    fn hex(b: &[u8]) -> String {
        b.iter().map(|x| format!("{x:02x}")).collect()
    }
}
