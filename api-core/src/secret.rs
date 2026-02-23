use base64::prelude::*;
use rand::Rng;
use serde::{Deserialize, Serialize};
use std::{
    fs::File,
    io::{self, Error, Read, Write},
    path::PathBuf,
    time::{Duration, SystemTime},
};

const SECRET_EXPIRY_TIMEOUT: Duration = Duration::from_secs(30 * 24 * 3600);

#[derive(Serialize, Deserialize)]
struct SecretKeeper {
    created: SystemTime,
    encoded: String,
}

impl SecretKeeper {
    fn new(created: SystemTime, encoded: String) -> Self {
        Self { created, encoded }
    }

    fn expired(&self) -> io::Result<bool> {
        if let Ok(duration_since) = self.created.elapsed() {
            Ok(duration_since > SECRET_EXPIRY_TIMEOUT)
        } else {
            Err(Error::other("Failed to get duration since secret creation"))
        }
    }

    fn get_secret(&self) -> io::Result<[u8; 32]> {
        if let Ok(secret) = BASE64_STANDARD.decode(&self.encoded) {
            let mut res = [0u8; 32];
            res.copy_from_slice(&secret);
            Ok(res)
        } else {
            Err(Error::other("Failed to decode preexisting secret"))
        }
    }
}

/// Returns an existing 32-byte secret from `keeper_path`, or generates and persists a new one.
/// The secret is rotated automatically after 30 days.
pub fn get_or_create_secret(keeper_path: &str) -> io::Result<[u8; 32]> {
    let path = PathBuf::from(keeper_path);

    if path.is_file() {
        let mut file = File::open(&path)?;
        let mut buf = String::new();
        file.read_to_string(&mut buf)?;
        let keeper = serde_json::from_str::<SecretKeeper>(&buf)?;

        if keeper.expired()? {
            generate_secret(File::create(keeper_path)?)
        } else {
            keeper.get_secret()
        }
    } else {
        generate_secret(File::create(keeper_path)?)
    }
}

fn generate_secret(mut file: File) -> io::Result<[u8; 32]> {
    let mut res = [0u8; 32];
    rand::rng().fill(&mut res);

    let encoded = BASE64_STANDARD.encode(res);
    let keeper = SecretKeeper::new(SystemTime::now(), encoded);
    file.write_all(serde_json::to_string(&keeper)?.as_bytes())?;
    Ok(res)
}
