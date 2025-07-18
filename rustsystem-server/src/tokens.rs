use std::{
    fs::File,
    io::{self, Error, ErrorKind, Read, Write},
    path::PathBuf,
    time::{Duration, SystemTime},
};

use axum::{
    extract::FromRequestParts,
    http::{StatusCode, request::Parts},
};
use axum_extra::extract::cookie;
use base64::prelude::*;
use chrono::Utc;
use jsonwebtoken::{DecodingKey, EncodingKey, Header, Validation, decode, encode};
use rand::Rng;
use serde::{Deserialize, Serialize};

use crate::{AppState, MUID, UUID, new_muid, new_uuid};

#[derive(Debug, Deserialize, Serialize)]
struct MeetingClaims {
    uuid: UUID,
    muid: MUID,
    is_host: bool,
    exp: usize,
}

fn create_meeting_jwt(uuid: UUID, muid: MUID, is_host: bool, secret: &[u8; 32]) -> String {
    let expiration = Utc::now()
        .checked_add_signed(chrono::Duration::minutes(15))
        .expect("valid timestamp")
        .timestamp() as usize;

    let claims = MeetingClaims {
        uuid,
        muid,
        is_host,
        exp: expiration,
    };

    encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(secret.as_ref()),
    )
    .unwrap()
}

pub fn new_meeting_jwt(secret: &[u8; 32]) -> (UUID, MUID, String) {
    let uuid = new_uuid();
    let muid = new_muid();

    (uuid, muid, create_meeting_jwt(uuid, muid, true, secret))
}

pub fn get_meeting_jwt(uuid: UUID, muid: MUID, is_host: bool, secret: &[u8; 32]) -> String {
    create_meeting_jwt(uuid, muid, is_host, secret)
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
    pub uuid: UUID,
    pub muid: MUID,
    pub is_host: bool,
}

impl FromRequestParts<AppState> for AuthUser {
    type Rejection = StatusCode;

    async fn from_request_parts(parts: &mut Parts, state: &AppState) -> Result<Self, StatusCode> {
        let cookie_header = parts
            .headers
            .get(axum::http::header::COOKIE)
            .ok_or(StatusCode::UNAUTHORIZED)?
            .to_str()
            .or(Err(StatusCode::UNAUTHORIZED))?;

        let mut cookie_iter = cookie::Cookie::split_parse(cookie_header);

        let mut access_token = None;
        while let Some(c) = cookie_iter.next() {
            if let Ok(cookie) = c {
                if cookie.name() == "access_token" {
                    access_token = Some(cookie.value().to_owned());
                    break;
                }
            }
        }

        let token_data = decode::<MeetingClaims>(
            &access_token.ok_or(StatusCode::UNAUTHORIZED)?,
            &DecodingKey::from_secret(state.secret.as_ref()),
            &Validation::default(),
        )
        .map_err(|_| StatusCode::UNAUTHORIZED)?;

        Ok(AuthUser {
            uuid: token_data.claims.uuid,
            muid: token_data.claims.muid,
            is_host: token_data.claims.is_host,
        })
    }
}
