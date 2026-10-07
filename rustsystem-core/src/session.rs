//! Session cookies, used identically by both services (`docs/PROTOCOL.md` §3).
//!
//! A session is a random token in an `HttpOnly` cookie; the service keeps `SHA-256(token)`
//! mapped to who it belongs to. The cookie is persistent (`Max-Age`), so it survives page
//! refreshes and browser restarts, and page scripts can never read it.

use std::time::Duration;

use axum_extra::extract::cookie::{Cookie, SameSite};

/// How long a login lasts. Meetings are pruned after the same time.
pub const SESSION_TTL: Duration = Duration::from_secs(12 * 60 * 60);

pub const SERVER_COOKIE: &str = "rs_session";
pub const TRUSTAUTH_COOKIE: &str = "ta_session";

/// `secure` should be true whenever the service is reached over HTTPS (always in production).
pub fn session_cookie(name: &'static str, token: String, secure: bool) -> Cookie<'static> {
    Cookie::build((name, token))
        .http_only(true)
        .same_site(SameSite::Strict)
        .path("/")
        .max_age(time::Duration::seconds(SESSION_TTL.as_secs() as i64))
        .secure(secure)
        .build()
}

/// A cookie that tells the browser to forget the session.
pub fn expired_cookie(name: &'static str) -> Cookie<'static> {
    Cookie::build((name, ""))
        .http_only(true)
        .same_site(SameSite::Strict)
        .path("/")
        .max_age(time::Duration::ZERO)
        .build()
}
