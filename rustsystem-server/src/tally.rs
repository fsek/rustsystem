//! Encrypted tally files (`docs/PROTOCOL.md` §8).
//!
//! When a round closes, its result is encrypted for the meeting's tally key and written to
//! `<meetings dir>/<meeting id>/tally-<UTC time>-<round>.enc`. The server holds only the public
//! key, so it can write these files but never read them. The host's browser (or the
//! `decrypt-tally` CLI) re-derives the private key from the meeting password.
//!
//! **Container, version 2:**
//!
//! | Offset | Size | Field |
//! |---|---|---|
//! | 0 | 5 | magic `RSTLY` |
//! | 5 | 1 | version `2` |
//! | 6 | 1 | KDF id `1` = Argon2id v1.3 |
//! | 7 | 16 | KDF salt |
//! | 23 | 4 | `t_cost` (u32 BE) |
//! | 27 | 4 | `m_cost` KiB (u32 BE) |
//! | 31 | 1 | `p_cost` |
//! | 32 | 32 | ephemeral X25519 public key |
//! | 64 | 12 | nonce |
//! | 76 | … | ChaCha20-Poly1305 ciphertext + tag, with bytes 0..32 as associated data |
//!
//! Key schedule: `HKDF-SHA256(salt = ephemeral pk, ikm = ECDH, info = "rustsystem-tally-v2")`
//! → 32-byte key ‖ 12-byte nonce.

use std::{
    io::Write,
    path::{Path, PathBuf},
};

use chacha20poly1305::{
    ChaCha20Poly1305, Key, Nonce,
    aead::{Aead, KeyInit, Payload},
};
use hkdf::Hkdf;
use serde::{Deserialize, Serialize};
use sha2::Sha256;
use x25519_dalek::{EphemeralSecret, PublicKey};

use rustsystem_core::{ApiError, ApiResult, secret::b64_decode};

use crate::state::{ClosedRound, Counts};

pub const MAGIC: &[u8; 5] = b"RSTLY";
pub const VERSION: u8 = 2;
pub const KDF_ARGON2ID: u8 = 1;
pub const HEADER_LEN: usize = 32;
pub const HKDF_INFO: &[u8] = b"rustsystem-tally-v2";

/// Argon2id parameters the browser used to derive the tally key from the meeting password.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Kdf {
    pub salt: [u8; 16],
    pub t_cost: u32,
    pub m_cost_kib: u32,
    pub p_cost: u8,
}

impl Kdf {
    /// Accepts parameters a browser can compute in seconds but that are still meaningfully
    /// expensive to brute-force. The frontend uses t=3, m=64 MiB, p=1.
    pub fn new(salt_b64: &str, t_cost: u32, m_cost_kib: u32, p_cost: u8) -> ApiResult<Self> {
        let salt: [u8; 16] = b64_decode(salt_b64)
            .and_then(|s| s.try_into().ok())
            .ok_or_else(|| ApiError::invalid_input("The KDF salt must be 16 bytes."))?;
        if !(1..=10).contains(&t_cost) || !(8 * 1024..=1024 * 1024).contains(&m_cost_kib) || !(1..=4).contains(&p_cost) {
            return Err(ApiError::invalid_input("The KDF parameters are out of range."));
        }
        Ok(Self { salt, t_cost, m_cost_kib, p_cost })
    }

    fn header(&self) -> [u8; HEADER_LEN] {
        let mut h = [0u8; HEADER_LEN];
        h[..5].copy_from_slice(MAGIC);
        h[5] = VERSION;
        h[6] = KDF_ARGON2ID;
        h[7..23].copy_from_slice(&self.salt);
        h[23..27].copy_from_slice(&self.t_cost.to_be_bytes());
        h[27..31].copy_from_slice(&self.m_cost_kib.to_be_bytes());
        h[31] = self.p_cost;
        h
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TallyKey {
    pub public_key: [u8; 32],
    pub kdf: Kdf,
}

impl TallyKey {
    pub fn new(public_key_b64: &str, kdf: Kdf) -> ApiResult<Self> {
        let public_key: [u8; 32] = b64_decode(public_key_b64)
            .and_then(|k| k.try_into().ok())
            .ok_or_else(|| ApiError::invalid_input("The tally public key must be 32 bytes."))?;
        Ok(Self { public_key, kdf })
    }
}

/// What a tally file contains once decrypted.
#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct TallyFile {
    pub meeting: String,
    pub round: String,
    pub round_id: uuid::Uuid,
    pub opened_at: String,
    pub closed_at: String,
    pub candidates: Vec<String>,
    pub score: Vec<usize>,
    pub blank: usize,
    pub counts: TallyFileCounts,
    pub participants: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct TallyFileCounts {
    pub eligible: usize,
    pub signed: Option<usize>,
    pub received: usize,
}

impl From<Counts> for TallyFileCounts {
    fn from(c: Counts) -> Self {
        Self { eligible: c.eligible, signed: c.signed, received: c.received }
    }
}

impl TallyFile {
    pub fn new(meeting_title: &str, closed: &ClosedRound, participants: Vec<String>) -> Self {
        Self {
            meeting: meeting_title.to_owned(),
            round: closed.spec.name.clone(),
            round_id: closed.id,
            opened_at: closed.opened_at.to_rfc3339(),
            closed_at: closed.closed_at.to_rfc3339(),
            candidates: closed.spec.candidates.clone(),
            score: closed.tally.score.clone(),
            blank: closed.tally.blank,
            counts: closed.counts.into(),
            participants,
        }
    }
}

pub fn encrypt(key: &TallyKey, plaintext: &[u8]) -> ApiResult<Vec<u8>> {
    let header = key.kdf.header();
    let ephemeral = EphemeralSecret::random_from_rng(rand_core::OsRng);
    let ephemeral_pk = PublicKey::from(&ephemeral);
    let shared = ephemeral.diffie_hellman(&PublicKey::from(key.public_key));

    let mut okm = [0u8; 44];
    Hkdf::<Sha256>::new(Some(ephemeral_pk.as_bytes()), shared.as_bytes())
        .expand(HKDF_INFO, &mut okm)
        .map_err(|_| ApiError::internal("HKDF expand failed"))?;
    // chacha20poly1305 0.10 still uses generic-array 0.14's deprecated constructors.
    #[allow(deprecated)]
    let (cipher_key, nonce) = (Key::from_slice(&okm[..32]), Nonce::from_slice(&okm[32..]));

    let ciphertext = ChaCha20Poly1305::new(cipher_key)
        .encrypt(nonce, Payload { msg: plaintext, aad: &header })
        .map_err(|_| ApiError::internal("tally encryption failed"))?;

    let mut out = Vec::with_capacity(HEADER_LEN + 32 + 12 + ciphertext.len());
    out.extend_from_slice(&header);
    out.extend_from_slice(ephemeral_pk.as_bytes());
    out.extend_from_slice(nonce);
    out.extend_from_slice(&ciphertext);
    Ok(out)
}

/// Encrypts `file` and writes it into `dir` atomically: a reader sees either no file or the
/// whole file, never a partial one. Returns the path written.
pub fn write(dir: &Path, key: &TallyKey, file: &TallyFile, closed: &ClosedRound) -> ApiResult<PathBuf> {
    let plaintext = serde_json::to_vec(file).map_err(ApiError::internal)?;
    let encrypted = encrypt(key, &plaintext)?;

    std::fs::create_dir_all(dir).map_err(|e| ApiError::internal(format!("creating {}: {e}", dir.display())))?;
    let name = format!(
        "tally-{}-{}.enc",
        closed.closed_at.format("%Y%m%dT%H%M%SZ"),
        &closed.id.simple().to_string()[..8]
    );
    let path = dir.join(&name);
    let tmp = dir.join(format!(".{name}.tmp"));

    let result = (|| {
        let mut f = std::fs::File::create(&tmp)?;
        f.write_all(&encrypted)?;
        f.sync_all()?;
        std::fs::rename(&tmp, &path)
    })();
    if let Err(e) = result {
        let _ = std::fs::remove_file(&tmp);
        return Err(ApiError::internal(format!("writing {}: {e}", path.display())));
    }
    Ok(path)
}

/// Every tally file in `dir`, oldest first, as `(filename, bytes)`.
pub fn read_all(dir: &Path) -> ApiResult<Vec<(String, Vec<u8>)>> {
    let entries = match std::fs::read_dir(dir) {
        Ok(entries) => entries,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(e) => return Err(ApiError::internal(format!("reading {}: {e}", dir.display()))),
    };
    let mut files = Vec::new();
    for entry in entries {
        let entry = entry.map_err(ApiError::internal)?;
        let name = entry.file_name().to_string_lossy().into_owned();
        if name.starts_with("tally-") && name.ends_with(".enc") {
            let bytes = std::fs::read(entry.path()).map_err(ApiError::internal)?;
            files.push((name, bytes));
        }
    }
    files.sort_by(|a, b| a.0.cmp(&b.0));
    Ok(files)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::{RoundSpec, Tally};
    use chrono::Utc;
    use x25519_dalek::x25519;

    const SCALAR: [u8; 32] = [7u8; 32];

    fn key() -> TallyKey {
        TallyKey {
            public_key: x25519(SCALAR, x25519_dalek::X25519_BASEPOINT_BYTES),
            kdf: Kdf { salt: *b"\x00\x11\x22\x33\x44\x55\x66\x77\x88\x99\xaa\xbb\xcc\xdd\xee\xff", t_cost: 3, m_cost_kib: 65536, p_cost: 1 },
        }
    }

    fn closed() -> ClosedRound {
        ClosedRound {
            id: uuid::Uuid::new_v4(),
            spec: RoundSpec::new("Chair", &["Anna".into(), "Bo".into()], 1, false).unwrap(),
            tally: Tally { score: vec![12, 9], blank: 2 },
            counts: Counts { eligible: 25, signed: Some(23), received: 23 },
            opened_at: Utc::now(),
            closed_at: Utc::now(),
        }
    }

    /// What the browser and `decrypt-tally` do, given the derived scalar.
    pub(crate) fn decrypt(file: &[u8], scalar: [u8; 32]) -> Result<Vec<u8>, ()> {
        let header = &file[..HEADER_LEN];
        let eph: [u8; 32] = file[HEADER_LEN..HEADER_LEN + 32].try_into().unwrap();
        let nonce = &file[HEADER_LEN + 32..HEADER_LEN + 44];
        let shared = x25519(scalar, eph);
        let mut okm = [0u8; 44];
        Hkdf::<Sha256>::new(Some(&eph), &shared).expand(HKDF_INFO, &mut okm).unwrap();
        #[allow(deprecated)]
        let cipher = ChaCha20Poly1305::new(Key::from_slice(&okm[..32]));
        #[allow(deprecated)]
        cipher
            .decrypt(Nonce::from_slice(nonce), Payload { msg: &file[HEADER_LEN + 44..], aad: header })
            .map_err(|_| ())
    }

    fn written(dir: &Path) -> (String, Vec<u8>) {
        let c = closed();
        write(dir, &key(), &TallyFile::new("Vårmöte", &c, vec!["Anna".into()]), &c).unwrap();
        let mut all = read_all(dir).unwrap();
        assert_eq!(all.len(), 1);
        all.pop().unwrap()
    }

    fn temp_dir() -> PathBuf {
        let d = std::env::temp_dir().join(format!("rustsystem-tally-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    #[test]
    fn header_layout() {
        let dir = temp_dir();
        let (_, file) = written(&dir);
        assert_eq!(&file[..5], b"RSTLY");
        assert_eq!(file[5], 2);
        assert_eq!(file[6], 1);
        assert_eq!(hex(&file[7..23]), "00112233445566778899aabbccddeeff");
        assert_eq!(u32::from_be_bytes(file[23..27].try_into().unwrap()), 3);
        assert_eq!(u32::from_be_bytes(file[27..31].try_into().unwrap()), 65536);
        assert_eq!(file[31], 1);
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn round_trip() {
        let dir = temp_dir();
        let (_, file) = written(&dir);
        let plain: TallyFile = serde_json::from_slice(&decrypt(&file, SCALAR).unwrap()).unwrap();
        assert_eq!(plain.meeting, "Vårmöte");
        assert_eq!(plain.candidates, vec!["Anna", "Bo"]);
        assert_eq!(plain.score, vec![12, 9]);
        assert_eq!(plain.blank, 2);
        assert_eq!(plain.counts, TallyFileCounts { eligible: 25, signed: Some(23), received: 23 });
        assert_eq!(plain.participants, vec!["Anna"]);
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn header_is_authenticated_and_key_matters() {
        let dir = temp_dir();
        let (_, mut file) = written(&dir);
        assert!(decrypt(&file, [9u8; 32]).is_err(), "wrong key");
        file[7] ^= 0xff;
        assert!(decrypt(&file, SCALAR).is_err(), "tampered salt");
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn filename_is_shell_safe_and_no_temp_left_behind() {
        let dir = temp_dir();
        let (name, _) = written(&dir);
        assert!(!name.contains(' ') && !name.contains(':') && !name.contains('+'), "{name}");
        let leftovers = std::fs::read_dir(&dir).unwrap().filter(|e| {
            e.as_ref().unwrap().file_name().to_string_lossy().ends_with(".tmp")
        });
        assert_eq!(leftovers.count(), 0);
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn unwritable_directory_is_an_error() {
        let dir = temp_dir();
        let blocker = dir.join("not-a-dir");
        std::fs::write(&blocker, b"").unwrap();
        let c = closed();
        assert!(write(&blocker, &key(), &TallyFile::new("M", &c, vec![]), &c).is_err());
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn kdf_parameters_are_bounded() {
        let salt = rustsystem_core::secret::b64_encode([0u8; 16]);
        assert!(Kdf::new(&salt, 3, 65536, 1).is_ok());
        assert!(Kdf::new(&salt, 0, 65536, 1).is_err());
        assert!(Kdf::new(&salt, 3, 1024, 1).is_err());
        assert!(Kdf::new(&salt, 3, 65536, 0).is_err());
        assert!(Kdf::new("short", 3, 65536, 1).is_err());
    }

    fn hex(b: &[u8]) -> String {
        b.iter().map(|x| format!("{x:02x}")).collect()
    }
}
