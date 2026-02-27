use axum::{
    Json,
    extract::FromRequestParts,
    http::{StatusCode, request::Parts},
};
use axum_extra::extract::{
    CookieJar,
    cookie::{self, Cookie},
};
use rustsystem_core::{APIError, APIErrorCode, APIErrorFinal, EndpointMeta};
use uuid::Uuid;

use crate::{AppState, MUuid, UUuid};

// TODO: Improve security by refreshing JWT or switching to server based sessions.
// The 12 hours is fine for now. Realistically, stealing JWTs over TLS is very difficult.
pub fn new_cookie(jwt: String, is_secure: bool) -> Cookie<'static> {
    Cookie::build(("rustsystem_access_token", jwt))
        .http_only(true)
        .same_site(cookie::SameSite::Strict)
        .path("/")
        .max_age(time::Duration::hours(12))
        .secure(is_secure)
        .into()
}

pub fn new_meeting_jwt(secret: &[u8; 32]) -> Result<(UUuid, MUuid, String), APIError> {
    let uuuid = Uuid::new_v4();
    let muuid = Uuid::new_v4();
    Ok((
        uuuid,
        muuid,
        rustsystem_core::tokens::encode_jwt(uuuid, muuid, true, secret, rustsystem_core::tokens::SERVER_ISSUER)?,
    ))
}

pub fn get_meeting_jwt(
    uuuid: UUuid,
    muuid: MUuid,
    is_host: bool,
    secret: &[u8; 32],
) -> Result<String, APIError> {
    rustsystem_core::tokens::encode_jwt(uuuid, muuid, is_host, secret, rustsystem_core::tokens::SERVER_ISSUER)
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
        // CookieJar extraction is infallible; match the Infallible variant exhaustively.
        let jar = match CookieJar::from_request_parts(parts, state).await {
            Ok(jar) => jar,
            Err(infallible) => match infallible {},
        };

        let endpoint = EndpointMeta {
            method: rustsystem_core::Method::from(parts.method.clone()),
            path: parts.uri.path().to_string(),
        };

        let access_token = jar
            .get("rustsystem_access_token")
            .ok_or_else(|| {
                APIError::from_error_code(APIErrorCode::AuthError)
                    .finalize(endpoint.clone())
                    .response()
            })?
            .value()
            .to_owned();

        let state_guard = {
            let guard = state
                .read()
                .map_err(|e| e.finalize(endpoint.clone()).response())?;
            guard.clone()
        };

        let claims = rustsystem_core::tokens::decode_jwt(&access_token, &state_guard.secret, rustsystem_core::tokens::SERVER_ISSUER)
            .map_err(|e| e.finalize(endpoint.clone()).response())?;

        // Verify the user still exists in the meeting. This implicitly revokes
        // tokens whenever a voter is removed or the meeting is closed — no
        // separate blocklist is needed. The block scope drops the lock guard
        // before we return so we don't hold it any longer than necessary.
        {
            let map = state_guard.meetings.read().await;
            let meeting = map.get(&claims.muuid).cloned().ok_or_else(|| {
                APIError::from_error_code(APIErrorCode::AuthError)
                    .finalize(endpoint.clone())
                    .response()
            })?;
            drop(map);
            if !meeting
                .voters
                .read()
                .await
                .contains_key(&claims.uuuid)
            {
                return Err(APIError::from_error_code(APIErrorCode::AuthError)
                    .finalize(endpoint)
                    .response());
            }
        }

        Ok(AuthUser {
            uuuid: claims.uuuid,
            muuid: claims.muuid,
            is_host: claims.is_host,
        })
    }
}
