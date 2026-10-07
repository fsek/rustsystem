//! Trustauth's public API, called by voters' browsers (`docs/PROTOCOL.md` §4.3, §5.2, §7).
//!
//! | Method | Path          | Auth       | Purpose                                   |
//! |--------|---------------|------------|-------------------------------------------|
//! | POST   | `/api/login`  | ticket     | Exchange a server-issued ticket for a session |
//! | POST   | `/api/logout` | session    | End the session                           |
//! | GET    | `/api/status` | session    | Is a round open, and am I already signed? |
//! | POST   | `/api/sign`   | session    | Blind-sign my ballot (once per round)     |

use std::time::Instant;

use axum::{
    extract::{FromRequestParts, State},
    http::{StatusCode, request::Parts},
};
use axum_extra::extract::CookieJar;
use serde::{Deserialize, Serialize};
use tracing::{info, warn};

use rustsystem_core::{
    ApiError, ApiResult, ErrorCode,
    extract::Json,
    internal::RoundId,
    secret::{b64_decode, b64_encode},
    session::{TRUSTAUTH_COOKIE, expired_cookie, session_cookie},
};

use crate::{
    AppState,
    state::{Identity, VoterStatus},
};

/// A request from a logged-in voter.
pub struct Voter(pub Identity);

impl FromRequestParts<AppState> for Voter {
    type Rejection = ApiError;

    async fn from_request_parts(parts: &mut Parts, app: &AppState) -> ApiResult<Self> {
        let jar = CookieJar::from_headers(&parts.headers);
        let token = jar.get(TRUSTAUTH_COOKIE).ok_or(ErrorCode::NotLoggedIn)?;
        app.state
            .session(token.value(), Instant::now())
            .map(Voter)
            .ok_or_else(|| ErrorCode::SessionExpired.into())
    }
}

#[derive(Deserialize)]
pub struct LoginRequest {
    ticket: String,
}

pub async fn login(
    State(app): State<AppState>,
    jar: CookieJar,
    Json(req): Json<LoginRequest>,
) -> ApiResult<(StatusCode, CookieJar)> {
    let (token, who) = app.state.redeem_ticket(&req.ticket, Instant::now())?;
    info!(meeting = %who.meeting, "Voter logged in to trustauth");
    let cookie = session_cookie(TRUSTAUTH_COOKIE, token, app.secure_cookies);
    Ok((StatusCode::NO_CONTENT, jar.add(cookie)))
}

pub async fn logout(State(app): State<AppState>, jar: CookieJar) -> (StatusCode, CookieJar) {
    if let Some(token) = jar.get(TRUSTAUTH_COOKIE) {
        app.state.logout(token.value());
    }
    (StatusCode::NO_CONTENT, jar.add(expired_cookie(TRUSTAUTH_COOKIE)))
}

pub async fn status(State(app): State<AppState>, Voter(who): Voter) -> Json<VoterStatus> {
    Json(app.state.voter_status(who))
}

#[derive(Deserialize)]
pub struct SignRequest {
    round: RoundId,
    /// base64url
    blinded: String,
}

#[derive(Serialize)]
pub struct SignResponse {
    /// base64url
    blind_sig: String,
}

/// Signs a blinded ballot. Trustauth learns only that this voter has now voted: it records
/// the voter ID and nothing about the blinded message or the signature, not even in logs.
pub async fn sign(
    State(app): State<AppState>,
    Voter(who): Voter,
    Json(req): Json<SignRequest>,
) -> ApiResult<Json<SignResponse>> {
    let blinded = b64_decode(&req.blinded)
        .filter(|b| b.len() == rustsystem_core::blind::SIGNATURE_LEN)
        .ok_or_else(|| ApiError::with_message(ErrorCode::MalformedBallot, "The blinded ballot has the wrong length."))?;

    let key = app.state.claim_signature(who, req.round)?;
    // RSA signing takes a millisecond or two; keep it off the async workers and outside the lock.
    let signed = tokio::task::spawn_blocking(move || key.blind_sign(&blinded))
        .await
        .map_err(ApiError::internal)?;

    match signed {
        Ok(blind_sig) => {
            info!(meeting = %who.meeting, "Signature issued");
            Ok(Json(SignResponse { blind_sig: b64_encode(blind_sig) }))
        }
        Err(e) => {
            app.state.release_signature(who, req.round);
            warn!(meeting = %who.meeting, "Blind signing failed: {e:?}");
            Err(ErrorCode::MalformedBallot.into())
        }
    }
}
