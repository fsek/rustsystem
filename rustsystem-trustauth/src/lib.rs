use rustsystem_core::{
    APIError, APIErrorCode,
    mtls::build_mtls_client,
};
use axum::{
    Router,
    http::{HeaderValue, Method, header},
};
use reqwest::{Client, Response};
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, sync::Arc};
use tokio::sync::RwLock as AsyncRwLock;
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
    pub keys: AuthenticationKeys,    // immutable after construction — no lock
    pub header: Vec<u8>,             // immutable after construction — no lock
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
        info!("Fetching voter");
        self.post("is-voter", &body)
            .await?
            .json::<IsVoterResponse>()
            .await
            .map(|r| r.is_voter)
            .map_err(|e| {
                error!("Failed to fetch voter: {e}");
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
            .map_err(|e| {
                error!("Failed to fetch voter: {e}");
                APIError::from_error_code(APIErrorCode::TrustAuthFetch)
            })
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

pub fn init_state() -> anyhow::Result<AppState> {
    let secret = rustsystem_core::secret::get_or_create_secret("/tmp/rustsystem-trustauth-secret")
        .map_err(|e| anyhow::anyhow!("Failed to load trustauth secret: {e}"))?;
    info!("Loaded trustauth secret");

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

pub fn app_public(state: AppState) -> Router {
    // Allow both the canonical origin and its localhost/127.0.0.1 counterpart,
    // since browsers may use either form even when pointing at the same host.
    let mut origins: Vec<HeaderValue> = vec![
        API_ENDPOINT_SERVER
            .parse()
            .expect("API_ENDPOINT_SERVER is not a valid header value"),
    ];
    let alt = if API_ENDPOINT_SERVER.contains("127.0.0.1") {
        API_ENDPOINT_SERVER.replace("127.0.0.1", "localhost")
    } else {
        API_ENDPOINT_SERVER.replace("localhost", "127.0.0.1")
    };
    if alt != API_ENDPOINT_SERVER
        && let Ok(v) = alt.parse()
    {
        origins.push(v);
    }

    let cors = CorsLayer::new()
        .allow_origin(AllowOrigin::list(origins))
        .allow_methods([Method::GET, Method::POST])
        .allow_headers([header::CONTENT_TYPE])
        .allow_credentials(true);

    Router::new()
        .nest("/api", public_routes())
        .layer(cors)
        .with_state(state)
}

pub fn app_internal(state: AppState) -> Router {
    Router::new()
        .nest("/server/api", internal_routes())
        .with_state(state)
}

/// Combines public and internal routers on a single `Router`. Used by
/// integration tests that run both services in the same process on a single port.
pub fn app_combined(state: AppState) -> Router {
    app_public(state.clone()).merge(app_internal(state))
}
