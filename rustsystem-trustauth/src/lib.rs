//! Trustauth: the service that knows *who* voters are and signs one ballot per voter per round,
//! without seeing it. The protocol is specified in `docs/PROTOCOL.md`.
//!
//! - [`state`] — what trustauth knows and every rule it enforces.
//! - [`api`] — the public API for browsers (port 2443).
//! - [`internal`] — the internal API for the server (port 2444, mTLS).
//! - [`config`] — runtime configuration.

use std::time::{Duration, Instant};

use axum::{
    Router,
    extract::DefaultBodyLimit,
    http::{HeaderValue, Method, header},
    routing::{delete, get, post},
};
use tower_http::cors::{AllowOrigin, CorsLayer};

use rustsystem_core::{
    internal::paths,
    limits::{MAX_BODY_BYTES, RateLimitLayer},
};

pub mod api;
pub mod config;
pub mod internal;
pub mod state;

#[derive(Clone)]
pub struct AppState {
    pub state: state::State,
    /// Mark cookies `Secure`; true whenever trustauth is reached over HTTPS.
    pub secure_cookies: bool,
}

impl AppState {
    pub fn new(secure_cookies: bool) -> Self {
        Self {
            state: state::State::default(),
            secure_cookies,
        }
    }

    /// Forgets expired sessions, tickets and stale rounds once a minute.
    pub fn spawn_pruning(&self) {
        let state = self.state.clone();
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(60));
            loop {
                interval.tick().await;
                state.prune(Instant::now());
            }
        });
    }
}

/// The browser-facing API.
///
/// `allowed_origins` are the server's public origins: the voter's page is served by the server
/// and calls trustauth cross-origin, with cookies.
pub fn public_router(
    app: AppState,
    allowed_origins: Vec<HeaderValue>,
    rate_limit: Option<RateLimitLayer>,
) -> Router {
    let mut api = Router::new()
        .route("/login", post(api::login))
        .route("/logout", post(api::logout))
        .route("/status", get(api::status))
        .route("/sign", post(api::sign));
    if let Some(layer) = rate_limit {
        api = api.layer(layer);
    }

    let cors = CorsLayer::new()
        .allow_origin(AllowOrigin::list(allowed_origins))
        .allow_methods([Method::GET, Method::POST])
        .allow_headers([header::CONTENT_TYPE])
        .allow_credentials(true);

    Router::new()
        .nest("/api", api)
        .layer(DefaultBodyLimit::max(MAX_BODY_BYTES))
        .layer(cors)
        .with_state(app)
}

/// The server-facing API. Only ever exposed behind mTLS.
pub fn internal_router(app: AppState) -> Router {
    Router::new()
        .route(paths::TICKETS, post(internal::issue_ticket))
        .route(paths::ROUNDS, post(internal::start_round))
        .route(
            &format!("{}/{{meeting}}", paths::ROUNDS),
            get(internal::round_status).delete(internal::drop_round),
        )
        .route(&format!("{}/{{meeting}}", paths::MEETINGS), delete(internal::drop_meeting))
        .route(
            &format!("{}/{{meeting}}/{{voter}}", paths::VOTERS),
            delete(internal::drop_voter),
        )
        .layer(DefaultBodyLimit::max(MAX_BODY_BYTES))
        .with_state(app)
}
