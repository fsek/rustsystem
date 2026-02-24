use std::io;

use rustsystem_core::{APIError, APIErrorCode, APIErrorFinal, EndpointMeta};
use axum::{
    Json,
    extract::FromRequestParts,
    http::{StatusCode, request::Parts},
};
use axum_extra::extract::{
    CookieJar,
    cookie::{self, Cookie},
};
use chrono::Utc;
use jsonwebtoken::{DecodingKey, EncodingKey, Header, Validation, decode, encode};
use serde::{Deserialize, Serialize};
use time::{self, OffsetDateTime};
use uuid::Uuid;

use crate::{AppState, MUuid, UUuid};

#[derive(Debug, Deserialize, Serialize)]
struct MeetingClaims {
    uuuid: Uuid,
    muuid: Uuid,
    is_host: bool,
    exp: usize,
}

// TODO: Improve security by refreshing JWT or switching to server based sessions.
// The 12 hours is fine for now. Realistically, stealing JWTs over TLS is very difficult.
fn create_meeting_jwt(
    uuuid: UUuid,
    muuid: MUuid,
    is_host: bool,
    secret: &[u8; 32],
) -> Result<String, jsonwebtoken::errors::Error> {
    let expiration = Utc::now()
        .checked_add_signed(chrono::Duration::hours(12))
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
}

pub fn new_cookie(jwt: String, is_secure: bool) -> Cookie<'static> {
    Cookie::build(("access_token", jwt))
        .http_only(true)
        .same_site(cookie::SameSite::Strict)
        .path("/")
        .expires(OffsetDateTime::now_utc().checked_add(time::Duration::hours(12)))
        .secure(is_secure)
        .max_age(time::Duration::hours(12))
        .into()
}

pub fn new_meeting_jwt(
    secret: &[u8; 32],
) -> Result<(UUuid, MUuid, String), jsonwebtoken::errors::Error> {
    let uuuid = Uuid::new_v4();
    let muuid = Uuid::new_v4();

    Ok((
        uuuid,
        muuid,
        create_meeting_jwt(uuuid, muuid, true, secret)?,
    ))
}

pub fn get_meeting_jwt(
    uuid: UUuid,
    muid: MUuid,
    is_host: bool,
    secret: &[u8; 32],
) -> Result<String, jsonwebtoken::errors::Error> {
    create_meeting_jwt(uuid, muid, is_host, secret)
}

pub fn get_secret() -> io::Result<[u8; 32]> {
    rustsystem_core::secret::get_or_create_secret("/tmp/rustsystem-server-secret")
}

pub struct AuthUser {
    pub uuuid: UUuid,
    pub muuid: MUuid,
    pub is_host: bool,
}

impl FromRequestParts<AppState> for AuthUser {
    type Rejection = (StatusCode, Json<APIErrorFinal>);

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        let jar = CookieJar::from_request_parts(parts, state)
            .await
            .expect("infallible");

        let endpoint = EndpointMeta {
            method: rustsystem_core::Method::from(parts.method.clone()),
            path: parts.uri.path().to_string(),
        };

        let access_token = jar
            .get("access_token")
            .ok_or(
                APIError::from_error_code(APIErrorCode::AuthError)
                    .finalize(endpoint.clone())
                    .response(),
            )?
            .value();

        let state_guard = {
            let guard = state
                .read()
                .map_err(|e| e.finalize(endpoint.clone()).response())?;
            guard.clone()
        };

        let token_data = decode::<MeetingClaims>(
            access_token,
            &DecodingKey::from_secret(state_guard.secret.as_ref()),
            &Validation::default(),
        )
        .map_err(|_| {
            APIError::from_error_code(APIErrorCode::AuthError)
                .finalize(endpoint.clone())
                .response()
        })?;

        // Verify the user still exists in the meeting. This implicitly revokes
        // tokens whenever a voter is removed or the meeting is closed — no
        // separate blocklist is needed. The block scope drops the lock guard
        // before we return so we don't hold it any longer than necessary.
        {
            let meetings = state_guard.meetings.lock().await;
            let meeting = meetings.get(&token_data.claims.muuid).ok_or(
                APIError::from_error_code(APIErrorCode::AuthError)
                    .finalize(endpoint.clone())
                    .response(),
            )?;
            if !meeting.voters.contains_key(&token_data.claims.uuuid) {
                return Err(APIError::from_error_code(APIErrorCode::AuthError)
                    .finalize(endpoint)
                    .response());
            }
        }

        Ok(AuthUser {
            uuuid: token_data.claims.uuuid,
            muuid: token_data.claims.muuid,
            is_host: token_data.claims.is_host,
        })
    }
}
