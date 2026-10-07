//! Who is making a request. A handler states its auth requirement in its signature:
//!
//! - [`Member`] — any logged-in voter of a meeting (hosts included);
//! - [`Host`] — a logged-in host;
//! - neither — anyone. Only creating a meeting, logging in, and submitting a ballot are open;
//!   the ballot handler deliberately has no way to learn who sent it.

use std::sync::Arc;

use axum::{extract::FromRequestParts, http::request::Parts};
use axum_extra::extract::CookieJar;

use rustsystem_core::{ApiError, ApiResult, ErrorCode, internal::VoterId, session::SERVER_COOKIE};

use crate::app::{AppState, Meeting};

pub struct Member {
    pub meeting: Arc<Meeting>,
    pub voter: VoterId,
    /// The session cookie's value, so logout can end exactly this session.
    pub token: String,
}

impl FromRequestParts<AppState> for Member {
    type Rejection = ApiError;

    async fn from_request_parts(parts: &mut Parts, app: &AppState) -> ApiResult<Self> {
        let jar = CookieJar::from_headers(&parts.headers);
        let token = jar.get(SERVER_COOKIE).ok_or(ErrorCode::NotLoggedIn)?.value().to_owned();
        let (meeting, voter) = app.session(&token).ok_or(ErrorCode::SessionExpired)?;
        let meeting = app.meeting(meeting).ok_or(ErrorCode::SessionExpired)?;
        Ok(Member { meeting, voter, token })
    }
}

pub struct Host(pub Member);

impl FromRequestParts<AppState> for Host {
    type Rejection = ApiError;

    async fn from_request_parts(parts: &mut Parts, app: &AppState) -> ApiResult<Self> {
        let member = Member::from_request_parts(parts, app).await?;
        let is_host = {
            let state = member.meeting.lock().await;
            state.voter(member.voter).ok_or(ErrorCode::SessionExpired)?.is_host
        };
        if !is_host {
            return Err(ErrorCode::NotHost.into());
        }
        Ok(Host(member))
    }
}
