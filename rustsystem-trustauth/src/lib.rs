use axum::http;
use axum::{
    Router,
    http::{HeaderValue, Method, header},
};
use reqwest::{Client, Response};
use rustsystem_core::{APIError, APIErrorCode, mtls::build_mtls_client};
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, sync::Arc, time::Duration};
use tokio::sync::RwLock as AsyncRwLock;
use tower_governor::{GovernorLayer, governor::GovernorConfigBuilder};
use tower_http::cors::{AllowOrigin, CorsLayer};
use tracing::{error, info};
use uuid::Uuid;
use zkryptium::{keys::pair::KeyPair, schemes::algorithms::BbsBls12381Sha256};

pub mod api;
pub mod api_server;
pub mod tokens;

use crate::{api::public_routes, api_server::internal_routes};

pub type AuthenticationKeys = KeyPair<BbsBls12381Sha256>;

pub const API_ENDPOINT_TO_SERVER: &str = env!("API_ENDPOINT_TRUSTAUTH_TO_SERVER");
pub const API_ENDPOINT_SERVER: &str = env!("API_ENDPOINT_SERVER");
pub const API_ENDPOINT_TRUSTAUTH: &str = env!("API_ENDPOINT_TRUSTAUTH");

/// Stored per voter after successful blind-sign registration.
pub struct VoterRegistration {
    pub token: Vec<u8>,
    pub blind_factor: Vec<u8>,
    pub commitment: Vec<u8>,
    pub context: serde_json::Value,
    pub signature: serde_json::Value,
}

/// Per-round state owned by trustauth.
pub struct RoundState {
    pub keys: AuthenticationKeys, // immutable after construction — no lock
    pub header: Vec<u8>,          // immutable after construction — no lock
    pub registered_voters: AsyncRwLock<HashMap<Uuid, VoterRegistration>>,
}

pub type ActiveRounds = Arc<AsyncRwLock<HashMap<Uuid, Arc<RoundState>>>>;

struct AppStateInternal {
    secret: [u8; 32],
    http_client: Client,
    rounds: ActiveRounds,
    server_url: String,
    is_secure: bool,
}

#[derive(Clone)]
pub struct AppState(Arc<AppStateInternal>);

impl AppState {
    pub fn secret(&self) -> &[u8; 32] {
        &self.0.secret
    }

    pub fn is_secure(&self) -> bool {
        self.0.is_secure
    }

    pub fn rounds_read(&self) -> ActiveRounds {
        self.0.rounds.clone()
    }

    pub fn rounds_write(&self) -> ActiveRounds {
        self.0.rounds.clone()
    }

    pub async fn get_round(&self, muuid: Uuid) -> Result<Arc<RoundState>, APIError> {
        self.0
            .rounds
            .read()
            .await
            .get(&muuid)
            .cloned()
            .ok_or_else(|| APIError::from_error_code(APIErrorCode::MUuidNotFound))
    }

    pub async fn get(&self, path: &str) -> Result<Response, APIError> {
        let url = format!("{}/trustauth/{path}", self.0.server_url);
        self.0
            .http_client
            .get(url)
            .send()
            .await
            .and_then(|r| r.error_for_status())
            .map_err(|_| APIError::from_error_code(APIErrorCode::TrustAuthFetch))
    }

    pub async fn vote_active(&self, muuid: Uuid) -> Result<bool, APIError> {
        let body = VoteActiveRequest { muuid };
        self.post("vote-active", &body)
            .await?
            .json::<VoteActiveResponse>()
            .await
            .map(|r| r.active)
            .map_err(|_| APIError::from_error_code(APIErrorCode::TrustAuthFetch))
    }

    pub async fn is_voter(&self, uuuid: Uuid, muuid: Uuid) -> Result<bool, APIError> {
        let body = IsVoterRequest { uuuid, muuid };
        self.post("is-voter", &body)
            .await?
            .json::<IsVoterResponse>()
            .await
            .map(|r| r.is_voter)
            .map_err(|e| {
                error!(muuid = %muuid, uuuid = %uuuid, "Failed to deserialize is-voter response: {e}");
                APIError::from_error_code(APIErrorCode::TrustAuthFetch)
            })
    }

    pub async fn post<B: serde::Serialize>(
        &self,
        path: &str,
        body: &B,
    ) -> Result<Response, APIError> {
        let url = format!("{}/trustauth/{path}", self.0.server_url);
        self.0
            .http_client
            .post(url)
            .json(body)
            .send()
            .await
            .and_then(|r| r.error_for_status())
            .map_err(|_| APIError::from_error_code(APIErrorCode::TrustAuthFetch))
    }
}

#[derive(Serialize)]
struct VoteActiveRequest {
    muuid: Uuid,
}

#[derive(Deserialize)]
struct VoteActiveResponse {
    active: bool,
}

#[derive(Serialize)]
struct IsVoterRequest {
    uuuid: Uuid,
    muuid: Uuid,
}

#[derive(Deserialize)]
struct IsVoterResponse {
    is_voter: bool,
}

pub fn init_state() -> Result<AppState, APIError> {
    use tracing::info;
    let secret = rustsystem_core::secret::generate_secret();
    info!("Trustauth state initialised");

    let http_client = build_mtls_client(
        include_bytes!("../../mtls/ca/ca.crt"),
        include_bytes!("../../mtls/trustauth/trustauth.crt"),
        include_bytes!("../../mtls/trustauth/trustauth.key"),
    )?;

    Ok(AppState(Arc::new(AppStateInternal {
        secret,
        http_client,
        rounds: Arc::new(AsyncRwLock::new(HashMap::new())),
        server_url: API_ENDPOINT_TO_SERVER.to_string(),
        is_secure: API_ENDPOINT_TRUSTAUTH.starts_with("https://"),
    })))
}

/// Creates an `AppState` suitable for integration tests: uses a plain HTTP client
/// (no mTLS) and accepts the server's base URL at runtime so tests can bind both
/// services to random ports.
pub fn new_test_state(server_url: impl Into<String>) -> AppState {
    AppState(Arc::new(AppStateInternal {
        secret: [0u8; 32],
        http_client: reqwest::Client::new(),
        rounds: Arc::new(AsyncRwLock::new(HashMap::new())),
        server_url: server_url.into(),
        is_secure: false,
    }))
}

fn build_cors() -> Result<CorsLayer, APIError> {
    // Allow both the canonical origin and its localhost/127.0.0.1 counterpart,
    // since browsers may use either form even when pointing at the same host.
    let origin: HeaderValue = API_ENDPOINT_SERVER.parse().map_err(|_| {
        APIError::new(
            APIErrorCode::InitError,
            "API_ENDPOINT_SERVER is not a valid CORS origin",
            500,
        )
    })?;
    let mut origins: Vec<HeaderValue> = vec![origin];

    let alt = (|| -> Option<String> {
        let uri: http::Uri = API_ENDPOINT_SERVER.parse().ok()?;
        let authority = uri.authority()?;
        let host = authority.host();
        let (old_host, new_host) = if host == "127.0.0.1" {
            ("127.0.0.1", "localhost")
        } else if host == "localhost" {
            ("localhost", "127.0.0.1")
        } else {
            return None;
        };
        let new_authority = authority.as_str().replacen(old_host, new_host, 1);
        let mut parts = uri.into_parts();
        parts.authority = new_authority.parse().ok();
        http::Uri::from_parts(parts).ok().map(|u| u.to_string())
    })()
    .unwrap_or_else(|| API_ENDPOINT_SERVER.to_string());
    if alt != API_ENDPOINT_SERVER
        && let Ok(v) = alt.parse()
    {
        origins.push(v);
    }

    Ok(CorsLayer::new()
        .allow_origin(AllowOrigin::list(origins))
        .allow_methods([Method::GET, Method::POST])
        .allow_headers([header::CONTENT_TYPE])
        .allow_credentials(true))
}

pub fn app_public(state: AppState) -> Result<Router, APIError> {
    // Rate limiting is only active in HTTPS (production) mode.
    // In HTTP mode (local dev and E2E tests) it is skipped so tests can send requests freely.
    let api = if state.is_secure() {
        let governor_conf = Arc::new(
            GovernorConfigBuilder::default()
                .per_second(10)
                .burst_size(30)
                .finish()
                .unwrap(),
        );
        // Periodically remove stale entries to prevent unbounded memory growth.
        let limiter = governor_conf.limiter().clone();
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(60));
            loop {
                interval.tick().await;
                limiter.retain_recent();
            }
        });
        public_routes().layer(GovernorLayer::new(governor_conf))
    } else {
        info!("Rate limiting disabled (non-HTTPS endpoint)");
        public_routes()
    };

    Ok(Router::new()
        .nest("/api", api)
        .layer(build_cors()?)
        .with_state(state))
}

pub fn app_internal(state: AppState) -> Router {
    Router::new()
        .nest("/server/api", internal_routes())
        .with_state(state)
}

/// Combines public and internal routers on a single `Router`. Used by
/// integration tests that run both services in the same process on a single port.
/// Rate limiting is intentionally omitted here — tests don't provide `ConnectInfo`.
pub fn app_combined(state: AppState) -> Result<Router, APIError> {
    let public = Router::new()
        .nest("/api", public_routes())
        .layer(build_cors()?)
        .with_state(state.clone());
    Ok(public.merge(app_internal(state)))
}
