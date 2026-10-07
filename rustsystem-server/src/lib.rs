//! The Rustsystem server: knows *what* was voted, never *who* voted it.
//! The protocol is specified in `docs/PROTOCOL.md`.
//!
//! - [`state`] — one meeting's state and every rule that governs it.
//! - [`ballot`] — what makes a ballot valid.
//! - [`tally`] — encrypted tally files.
//! - [`app`] — meetings, sessions and live-update plumbing; [`auth`] — who is asking.
//! - [`api`] — the HTTP handlers; [`trustauth`] — the client for trustauth.
//!
//! # API
//!
//! Every error is `{"code": "...", "message": "..."}` (`rustsystem_core::error`).
//!
//! | Method | Path | Auth | Handler |
//! |---|---|---|---|
//! | GET | `/api/config` | – | [`api::meeting::config`] |
//! | POST | `/api/meetings` | – (strict rate limit) | [`api::meeting::create`] |
//! | POST | `/api/login` | invite | [`api::meeting::login`] |
//! | POST | `/api/logout` | member | [`api::meeting::logout`] |
//! | POST | `/api/trustauth-ticket` | member | [`api::meeting::trustauth_ticket`] |
//! | GET | `/api/session` | member | [`api::meeting::session`] |
//! | GET | `/api/meeting` | member | [`api::meeting::get`] |
//! | GET | `/api/meeting/events` | member | [`api::events::stream`] (SSE) |
//! | POST | `/api/ballot` | **none, by design** | [`api::ballot::submit`] |
//! | GET | `/api/host/voters` | host | [`api::voters::list`] |
//! | POST | `/api/host/voters` | host | [`api::voters::add`] |
//! | DELETE | `/api/host/voters` | host | [`api::voters::remove_all`] |
//! | POST | `/api/host/voters/{id}/reset-invite` | host | [`api::voters::reset_invite`] |
//! | DELETE | `/api/host/voters/{id}` | host | [`api::voters::remove`] |
//! | GET | `/api/host/round` | host | [`api::round::status`] |
//! | POST | `/api/host/round` | host | [`api::round::start`] |
//! | DELETE | `/api/host/round` | host | [`api::round::reset`] |
//! | POST | `/api/host/round/close` | host | [`api::round::close`] |
//! | GET | `/api/host/tally-files` | host | [`api::round::tally_files`] |
//! | GET | `/api/host/agenda` | host | [`api::agenda::source`] |
//! | PUT | `/api/host/agenda` | host | [`api::agenda::set`] |
//! | DELETE | `/api/host/agenda` | host | [`api::agenda::clear`] |
//! | PUT | `/api/host/agenda/current` | host | [`api::agenda::go_to`] |
//! | GET | `/api/host/attendance` | host | [`api::agenda::attendance_log`] |
//! | POST | `/api/host/attendance` | host | [`api::agenda::take_attendance`] |
//! | DELETE | `/api/host/meeting` | host | [`api::meeting::close`] |
//!
//! Everything outside `/api` serves the frontend.

use std::path::Path;

use axum::{
    Router,
    extract::DefaultBodyLimit,
    routing::{delete, get, post, put},
};
use tower_http::services::{ServeDir, ServeFile};

use rustsystem_core::{
    ApiError, ErrorCode,
    limits::{MAX_BODY_BYTES, RateLimitLayer},
};

pub mod agenda;
pub mod api;
pub mod app;
pub mod auth;
pub mod ballot;
pub mod config;
pub mod invite;
pub mod state;
pub mod tally;
pub mod trustauth;

pub use app::{AppState, Settings};

#[derive(Default)]
pub struct RateLimits {
    /// Every `/api` route.
    pub general: Option<RateLimitLayer>,
    /// `POST /api/meetings`, on top of `general`.
    pub create_meeting: Option<RateLimitLayer>,
}

pub fn api_router(limits: RateLimits) -> Router<AppState> {
    use api::{agenda, ballot, events, meeting, round, voters};

    let mut create = post(meeting::create);
    if let Some(layer) = limits.create_meeting {
        create = create.layer(layer);
    }

    let host = Router::new()
        .route("/voters", get(voters::list).post(voters::add).delete(voters::remove_all))
        .route("/voters/{id}", delete(voters::remove))
        .route("/voters/{id}/reset-invite", post(voters::reset_invite))
        .route("/round", get(round::status).post(round::start).delete(round::reset))
        .route("/round/close", post(round::close))
        .route("/tally-files", get(round::tally_files))
        .route("/agenda", get(agenda::source).put(agenda::set).delete(agenda::clear))
        .route("/agenda/current", put(agenda::go_to))
        .route("/attendance", get(agenda::attendance_log).post(agenda::take_attendance))
        .route("/meeting", delete(meeting::close));

    let mut api = Router::new()
        .route("/config", get(meeting::config))
        .route("/meetings", create)
        .route("/login", post(meeting::login))
        .route("/logout", post(meeting::logout))
        .route("/trustauth-ticket", post(meeting::trustauth_ticket))
        .route("/session", get(meeting::session))
        .route("/meeting", get(meeting::get))
        .route("/meeting/events", get(events::stream))
        .route("/ballot", post(ballot::submit))
        .nest("/host", host)
        .fallback(|| async { ApiError::new(ErrorCode::NotFound) });
    if let Some(layer) = limits.general {
        api = api.layer(layer);
    }
    api.layer(DefaultBodyLimit::max(MAX_BODY_BYTES))
}

/// The whole public service: the API under `/api`, and the frontend from `frontend_dir`
/// (any unknown path gets `index.html`, so client-side routes work on reload).
pub fn router(app: AppState, frontend_dir: &Path, limits: RateLimits) -> Router {
    let frontend = ServeDir::new(frontend_dir).fallback(ServeFile::new(frontend_dir.join("index.html")));
    Router::new()
        .nest("/api", api_router(limits))
        .fallback_service(frontend)
        .with_state(app)
}
