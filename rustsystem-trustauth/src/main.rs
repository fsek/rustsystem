use api_core::{
    APIError, APIErrorCode,
    mtls::{build_mtls_client, build_mtls_server_config},
};
use axum::{
    Router,
    http::{HeaderValue, Method, header},
};
use axum_server::tls_rustls::RustlsConfig;
use tower_http::cors::{AllowOrigin, CorsLayer};

mod api;
mod api_server;
mod tokens;
use reqwest::{Client, Response};
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, net::SocketAddr, sync::Arc};
use tokio::sync::Mutex;
use tracing::{error, info, level_filters::LevelFilter};
use tracing_subscriber::EnvFilter;
use uuid::Uuid;
use zkryptium::{keys::pair::KeyPair, schemes::algorithms::BbsBls12381Sha256};

use crate::{api::public_routes, api_server::internal_routes};

pub type AuthenticationKeys = KeyPair<BbsBls12381Sha256>;

const API_ENDPOINT_TO_SERVER: &str = env!("API_ENDPOINT_TRUSTAUTH_TO_SERVER");
const API_ENDPOINT_SERVER: &str = env!("API_ENDPOINT_SERVER");

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
    pub keys: AuthenticationKeys,
    pub header: Vec<u8>,
    pub registered_voters: HashMap<Uuid, VoterRegistration>,
}

struct AppStateInternal {
    secret: [u8; 32],
    mtls_client: Client,
    rounds: Arc<Mutex<HashMap<Uuid, RoundState>>>,
}

#[derive(Clone)]
pub struct AppState(Arc<AppStateInternal>);

impl AppState {
    pub fn secret(&self) -> &[u8; 32] {
        &self.0.secret
    }

    pub fn rounds(&self) -> Arc<Mutex<HashMap<Uuid, RoundState>>> {
        self.0.rounds.clone()
    }

    pub async fn get(&self, path: &str) -> Result<Response, APIError> {
        self.0
            .mtls_client
            .get(format!("{API_ENDPOINT_TO_SERVER}/trustauth/{path}"))
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
        self.0
            .mtls_client
            .post(format!("{API_ENDPOINT_TO_SERVER}/trustauth/{path}"))
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

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env().add_directive(LevelFilter::INFO.into()))
        .init();

    let secret = api_core::secret::get_or_create_secret("/tmp/rustsystem-trustauth-secret")
        .map_err(|e| anyhow::anyhow!("Failed to load trustauth secret: {e}"))?;
    info!("Loaded trustauth secret");

    let mtls_client = build_mtls_client("trustauth")?;

    let state = AppState(Arc::new(AppStateInternal {
        secret,
        mtls_client,
        rounds: Arc::new(Mutex::new(HashMap::new())),
    }));

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
    if alt != API_ENDPOINT_SERVER {
        if let Ok(v) = alt.parse() {
            origins.push(v);
        }
    }

    let cors = CorsLayer::new()
        .allow_origin(AllowOrigin::list(origins))
        .allow_methods([Method::GET, Method::POST])
        .allow_headers([header::CONTENT_TYPE])
        .allow_credentials(true);

    let app_public = Router::new()
        .nest("/api", public_routes())
        .layer(cors)
        .with_state(state.clone());

    let app_internal = Router::new()
        .nest("/server/api", internal_routes())
        .with_state(state);

    let addr_public = SocketAddr::from(([0, 0, 0, 0], 2443));
    let addr_internal = SocketAddr::from(([0, 0, 0, 0], 2444));

    let tls_config = build_mtls_server_config(
        "mtls/trustauth/trustauth.crt",
        "mtls/trustauth/trustauth.key",
        "mtls/ca/ca.crt",
    )
    .unwrap();

    info!("Running trustauth server on {addr_public}");
    let public_serve = axum_server::bind(addr_public).serve(app_public.into_make_service());
    let internal_serve = axum_server::bind_rustls(
        addr_internal,
        RustlsConfig::from_config(std::sync::Arc::new(tls_config)),
    )
    .serve(app_internal.into_make_service());

    let (internal_res, public_res) = tokio::try_join!(internal_serve, public_serve)?;
    Ok(())
}
