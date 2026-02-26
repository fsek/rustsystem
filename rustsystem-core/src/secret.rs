use base64::prelude::*;
use rand::Rng;
use serde::{Deserialize, Serialize};
use std::{
    fs::File,
    io::{Read, Write},
    path::PathBuf,
    time::{Duration, SystemTime},
};

use crate::{APIError, APIErrorCode};

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

    fn expired(&self) -> Result<bool, APIError> {
        self.created
            .elapsed()
            .map(|d| d > SECRET_EXPIRY_TIMEOUT)
            .map_err(|_| APIError::new(APIErrorCode::InitError, "Failed to get duration since secret creation", 500))
    }

    fn get_secret(&self) -> Result<[u8; 32], APIError> {
        let decoded = BASE64_STANDARD
            .decode(&self.encoded)
            .map_err(|_| APIError::new(APIErrorCode::InitError, "Failed to decode preexisting secret", 500))?;
        decoded
            .try_into()
            .map_err(|_| APIError::new(APIErrorCode::InitError, "Secret has unexpected length", 500))
    }
}

/// Returns an existing 32-byte secret from `keeper_path`, or generates and persists a new one.
/// The secret is rotated automatically after 30 days.
pub fn get_or_create_secret(keeper_path: &str) -> Result<[u8; 32], APIError> {
    let path = PathBuf::from(keeper_path);

    if path.is_file() {
        let mut file = File::open(&path)
            .map_err(|_| APIError::from_error_code(APIErrorCode::InitError))?;
        let mut buf = String::new();
        file.read_to_string(&mut buf)
            .map_err(|_| APIError::from_error_code(APIErrorCode::InitError))?;
        let keeper = serde_json::from_str::<SecretKeeper>(&buf)
            .map_err(|_| APIError::from_error_code(APIErrorCode::InitError))?;

        if keeper.expired()? {
            generate_secret(
                File::create(keeper_path)
                    .map_err(|_| APIError::from_error_code(APIErrorCode::InitError))?,
            )
        } else {
            keeper.get_secret()
        }
    } else {
        generate_secret(
            File::create(keeper_path)
                .map_err(|_| APIError::from_error_code(APIErrorCode::InitError))?,
        )
    }
}

fn generate_secret(mut file: File) -> Result<[u8; 32], APIError> {
    let mut res = [0u8; 32];
    rand::rng().fill(&mut res);

    let encoded = BASE64_STANDARD.encode(res);
    let keeper = SecretKeeper::new(SystemTime::now(), encoded);
    file.write_all(
        serde_json::to_string(&keeper)
            .map_err(|_| APIError::from_error_code(APIErrorCode::InitError))?
            .as_bytes(),
    )
    .map_err(|_| APIError::from_error_code(APIErrorCode::InitError))?;
    Ok(res)
}
