use std::{
    fs::File,
    io::{self, Error, Read, Write},
    path::PathBuf,
    time::{Duration, SystemTime},
};

use api_core::{APIError, APIErrorCode};
use api_derive::APIEndpointError;
use axum::{
    Json,
    extract::FromRequestParts,
    http::{StatusCode, request::Parts},
};
use axum_extra::extract::CookieJar;
use base64::prelude::*;
use chrono::Utc;
use jsonwebtoken::{DecodingKey, EncodingKey, Header, Validation, decode, encode};
use rand::Rng;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{AppState, MUuid, UUuid};

#[derive(Debug, Deserialize, Serialize)]
struct MeetingClaims {
    uuuid: Uuid,
    muuid: Uuid,
    is_host: bool,
    exp: usize,
}

fn create_meeting_jwt(uuuid: UUuid, muuid: MUuid, is_host: bool, secret: &[u8; 32]) -> String {
    let expiration = Utc::now()
        .checked_add_signed(chrono::Duration::minutes(15))
        .expect("valid timestamp")
        .timestamp() as usize;

    let claims = MeetingClaims {
        uuuid,
        muuid,
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

pub fn new_meeting_jwt(secret: &[u8; 32]) -> (UUuid, MUuid, String) {
    let uuuid = Uuid::new_v4();
    let muuid = Uuid::new_v4();

    (uuuid, muuid, create_meeting_jwt(uuuid, muuid, true, secret))
}

pub fn get_meeting_jwt(uuid: UUuid, muid: MUuid, is_host: bool, secret: &[u8; 32]) -> String {
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
            Err(Error::other("Failed to get duration since secret creation"))
        }
    }

    pub fn get_secret(&self) -> io::Result<[u8; 32]> {
        if let Ok(secret) = BASE64_STANDARD.decode(&self.encoded) {
            let mut res = [0u8; 32];
            res.copy_from_slice(&secret);
            Ok(res)
        } else {
            Err(Error::other("Failed to decode preexisting secret"))
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
    let encoded = BASE64_STANDARD.encode(res);
    let keeper = SecretKeeper::new(SystemTime::now(), encoded);
    keeper_file.write_all(serde_json::to_string(&keeper)?.as_bytes())?;
    Ok(res)
}

pub struct AuthUser {
    pub uuuid: UUuid,
    pub muuid: MUuid,
    pub is_host: bool,
}

#[derive(APIEndpointError)]
#[api(endpoint(method = "-" path = "-"))]
pub enum AuthError {
    #[api(code = APIErrorCode::AuthError, status = 401)]
    AuthError,

    #[api(code = APIErrorCode::InvalidUUuid, status = 400)]
    InvalidUUuid,

    #[api(code = APIErrorCode::InvalidMUuid, status = 400)]
    InvalidMUuid,
}

impl FromRequestParts<AppState> for AuthUser {
    type Rejection = (StatusCode, Json<APIError>);

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        let jar = CookieJar::from_request_parts(parts, state)
            .await
            .expect("infallible");

        let access_token = jar
            .get("access_token")
            .ok_or(<AuthError as Into<APIError>>::into(AuthError::AuthError).finalize())?
            .value();

        let token_data = decode::<MeetingClaims>(
            &access_token,
            &DecodingKey::from_secret(state.secret.as_ref()),
            &Validation::default(),
        )
        .map_err(|_| <AuthError as Into<APIError>>::into(AuthError::AuthError).finalize())?;

        Ok(AuthUser {
            // TODO: Error handling
            uuuid: token_data.claims.uuuid,
            muuid: token_data.claims.muuid,
            is_host: token_data.claims.is_host,
        })
    }
}
