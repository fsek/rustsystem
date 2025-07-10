use std::{
    fs::File,
    io::{self, Error, ErrorKind, Read, Write},
    path::PathBuf,
    time::{Duration, SystemTime},
};

use axum::{
    extract::{FromRequestParts, State},
    http::{StatusCode, request::Parts},
};
use base64::prelude::*;
use chrono::Utc;
use jsonwebtoken::{DecodingKey, EncodingKey, Header, Validation, decode, encode};
use rand::Rng;
use serde::{Deserialize, Serialize};

use crate::AppState;

#[derive(Debug, Deserialize, Serialize)]
struct Claims {
    sub: String,
    exp: usize,
}

pub fn generate_jwt(user_id: &str, secret: &[u8; 32]) -> String {
    let expiration = Utc::now()
        .checked_add_signed(chrono::Duration::minutes(15))
        .expect("valid timestamp")
        .timestamp() as usize;

    let claims = Claims {
        sub: user_id.to_owned(),
        exp: expiration,
    };

    encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(secret.as_ref()),
    )
    .unwrap()
}

const KEEPER_PATH: &str = "/tmp/rustsystem-secret";
const SECRET_EXPITY_TIMEOUT: Duration = Duration::from_secs(30 * 24 * 3600);

#[derive(Serialize, Deserialize)]
struct SecretKeeper {
    created: SystemTime,
    encoded: String,
}
impl SecretKeeper {
    pub fn new(created: SystemTime, encoded: String) -> Self {
        Self { created, encoded }
    }

    pub fn expired(&self) -> io::Result<bool> {
        if let Ok(duration_since) = self.created.elapsed() {
            Ok(duration_since > SECRET_EXPITY_TIMEOUT)
        } else {
            Err(Error::new(
                ErrorKind::Other,
                "Failed to get duration since secret creation",
            ))
        }
    }

    pub fn get_secret(&self) -> io::Result<[u8; 32]> {
        if let Ok(secret) = BASE64_STANDARD.decode(&self.encoded) {
            let mut res = [0u8; 32];
            res.copy_from_slice(&secret);
            Ok(res)
        } else {
            Err(Error::new(
                ErrorKind::Other,
                "Failed to decode preexisting secret",
            ))
        }
    }
}

pub fn get_secret() -> io::Result<[u8; 32]> {
    let keeper_path = PathBuf::from(KEEPER_PATH);

    if keeper_path.is_file() {
        let mut keeper_file = File::open(keeper_path)?;
        let mut keeper_buf = String::new();
        keeper_file.read_to_string(&mut keeper_buf)?;
        let keeper = serde_json::from_str::<SecretKeeper>(&keeper_buf)?;

        if keeper.expired()? {
            // Overwrite existing secret
            generate_secret(keeper_file)
        } else {
            // Use preexisting secret
            keeper.get_secret()
        }
    } else {
        // generate new secret
        generate_secret(File::create(KEEPER_PATH)?)
    }
}

fn generate_secret(mut keeper_file: File) -> io::Result<[u8; 32]> {
    let mut res = [0u8; 32];
    rand::rng().fill(&mut res);

    // Encode and write to a keeper file.
    let encoded = BASE64_STANDARD.encode(&res);
    let keeper = SecretKeeper::new(SystemTime::now(), encoded);
    keeper_file.write_all(serde_json::to_string(&keeper)?.as_bytes())?;
    Ok(res)
}

pub struct AuthUser {
    pub user_id: String,
}

impl FromRequestParts<AppState> for AuthUser {
    type Rejection = StatusCode;

    async fn from_request_parts(parts: &mut Parts, state: &AppState) -> Result<Self, StatusCode> {
        let token = parts
            .headers
            .get("cookie")
            .and_then(|h| h.to_str().ok())
            .and_then(|cookie_header| {
                cookie_header
                    .split("; ")
                    .find_map(|c| c.strip_prefix("access_token="))
            })
            .ok_or(StatusCode::UNAUTHORIZED)?;

        println!("Found token {token}");

        let token_data = decode::<Claims>(
            token,
            &DecodingKey::from_secret(state.secret.as_ref()),
            &Validation::default(),
        )
        .map_err(|_| StatusCode::UNAUTHORIZED)?;

        Ok(AuthUser {
            user_id: token_data.claims.sub,
        })
    }
}
