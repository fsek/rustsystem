use std::time::{Duration, SystemTime, UNIX_EPOCH};

use jsonwebtoken::{DecodingKey, EncodingKey, Header, Validation, decode, encode};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{APIError, APIErrorCode};

pub const SERVER_ISSUER: &str = "rustsystem-server";
pub const TRUSTAUTH_ISSUER: &str = "rustsystem-trustauth";

#[derive(Debug, Serialize, Deserialize)]
pub struct MeetingClaims {
    pub iss: String,
    pub uuuid: Uuid,
    pub muuid: Uuid,
    pub is_host: bool,
    pub exp: u64,
}

/// Encodes a 12-hour JWT for the given voter/meeting pair.
/// `issuer` should be one of [`SERVER_ISSUER`] or [`TRUSTAUTH_ISSUER`].
pub fn encode_jwt(
    uuuid: Uuid,
    muuid: Uuid,
    is_host: bool,
    secret: &[u8; 32],
    issuer: &str,
) -> Result<String, APIError> {
    let exp = SystemTime::now()
        .checked_add(Duration::from_secs(12 * 3600))
        .ok_or_else(|| APIError::from_error_code(APIErrorCode::TimestampError))?
        .duration_since(UNIX_EPOCH)
        .map_err(|_| APIError::from_error_code(APIErrorCode::TimestampError))?
        .as_secs();

    let claims = MeetingClaims { iss: issuer.to_owned(), uuuid, muuid, is_host, exp };

    encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(secret.as_ref()),
    )
    .map_err(|_| APIError::from_error_code(APIErrorCode::CryptoError))
}

/// Decodes and validates a JWT, rejecting tokens not issued by `expected_issuer`.
pub fn decode_jwt(
    token: &str,
    secret: &[u8; 32],
    expected_issuer: &str,
) -> Result<MeetingClaims, APIError> {
    let mut validation = Validation::default();
    validation.set_issuer(&[expected_issuer]);
    // Require `iss` to be present in the token, not just validate its value if present.
    validation.required_spec_claims.insert("iss".to_owned());

    decode::<MeetingClaims>(
        token,
        &DecodingKey::from_secret(secret.as_ref()),
        &validation,
    )
    .map(|data| data.claims)
    .map_err(|_| APIError::from_error_code(APIErrorCode::AuthError))
}
