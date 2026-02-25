use rustsystem_core::{APIError, APIErrorCode, mtls::build_mtls_client};
use axum::Router;
use invite_auth::InviteAuthority;
use reqwest::Client;
use serde::Deserialize;
use std::{
    collections::HashMap,
    sync::{
        Arc, RwLock, RwLockReadGuard,
        atomic::{AtomicBool, Ordering},
    },
    time::SystemTime,
};
use tokens::{AuthUser, get_secret};
use tokio::sync::RwLock as AsyncRwLock;
use tower_http::services::{ServeDir, ServeFile};
use tracing::info;
use zkryptium::bbsplus::keys::BBSplusPublicKey;

mod admin_auth;
pub mod api;
use api::api_routes;
pub mod vote_auth;
use vote_auth::VoteAuthority;
mod invite_auth;
pub mod tokens;

pub mod api_trustauth;
use api_trustauth::api_trustauth_routes;

mod proof;
pub mod tally_encrypt;

use uuid::Uuid;

use crate::admin_auth::AdminAuthority;

type MUuid = Uuid;
type UUuid = Uuid;

/// NOTE: The API_ENDPOINT environmental variable must be set at compile time!
const API_ENDPOINT: &str = env!("API_ENDPOINT_SERVER");
const API_ENDPOINT_SERVER_TO_TRUSTAUTH: &str = env!("API_ENDPOINT_SERVER_TO_TRUSTAUTH");

#[derive(Debug)]
pub struct Voter {
    name: String,
    logged_in: bool,
    is_host: bool,
    registered_at: SystemTime,
}

/// Each authority is wrapped in its own `AsyncRwLock` so concurrent requests can hold
/// independent field locks rather than serialising on a single map-level lock.
/// `title` and `start_time` are immutable after construction and need no lock.
/// `locked` is a simple boolean that only needs atomic access.
pub struct Meeting {
    pub title: String,
    pub start_time: SystemTime,
    pub locked: AtomicBool,
    pub voters: AsyncRwLock<HashMap<Uuid, Voter>>,
    pub vote_auth: AsyncRwLock<VoteAuthority>,
    pub invite_auth: AsyncRwLock<InviteAuthority>,
    pub admin_auth: AsyncRwLock<AdminAuthority>,
}

impl Meeting {
    pub fn new(title: String, start_time: SystemTime, voters: HashMap<Uuid, Voter>) -> Self {
        Self {
            title,
            start_time,
            locked: AtomicBool::new(false),
            voters: AsyncRwLock::new(voters),
            vote_auth: AsyncRwLock::new(VoteAuthority::new()),
            invite_auth: AsyncRwLock::new(InviteAuthority::new()),
            admin_auth: AsyncRwLock::new(AdminAuthority::new()),
        }
    }

    pub fn unlock(&self) {
        self.locked.store(false, Ordering::Relaxed);
    }
}

/// The outer `AsyncRwLock` is held in *read* mode for almost all operations — just long enough to
/// clone the `Arc<Meeting>` — and in *write* mode only when creating or closing a meeting.
pub type ActiveMeetings = Arc<AsyncRwLock<HashMap<MUuid, Arc<Meeting>>>>;

#[derive(Clone)]
pub struct AppStateInternal {
    secret: [u8; 32],
    meetings: ActiveMeetings,
    // decides whether cookies should be sent as secure (i.e. require https). This should be true
    // for prod and false for dev
    is_secure: bool,
    trustauth_client: Client,
}

#[derive(Deserialize)]
struct StartRoundResponse {
    pub_key_bytes: Vec<u8>,
}

#[derive(Clone)]
pub struct AppState(Arc<RwLock<AppStateInternal>>);
impl AppState {
    pub fn read(&self) -> Result<RwLockReadGuard<'_, AppStateInternal>, APIError> {
        self.0
            .read()
            .map_err(|_e| APIError::from_error_code(APIErrorCode::StateCurrupt))
    }

    /// Use when you need to look up or read from an existing meeting.
    /// Callers should call `.read().await` on the returned Arc.
    pub fn meetings_read(&self) -> Result<ActiveMeetings, APIError> {
        let guard = self.read()?;
        Ok(guard.meetings.clone())
    }

    /// Use when you need to insert or remove a meeting from the map.
    /// Callers should call `.write().await` on the returned Arc.
    pub fn meetings_write(&self) -> Result<ActiveMeetings, APIError> {
        let guard = self.read()?;
        Ok(guard.meetings.clone())
    }

    /// Look up a meeting by MUUID, returning a cloned `Arc<Meeting>` that can be used
    /// after the outer map lock has been released.
    pub async fn get_meeting(&self, muuid: MUuid) -> Result<Arc<Meeting>, APIError> {
        self.meetings_read()?
            .read()
            .await
            .get(&muuid)
            .cloned()
            .ok_or_else(|| APIError::from_error_code(APIErrorCode::MUuidNotFound))
    }

    pub async fn start_round_on_trustauth(
        &self,
        muuid: MUuid,
        name: &str,
    ) -> Result<BBSplusPublicKey, APIError> {
        #[derive(serde::Serialize)]
        struct StartRoundRequest<'a> {
            muuid: MUuid,
            name: &'a str,
        }

        let client = {
            let guard = self.read()?;
            guard.trustauth_client.clone()
        };

        let resp = client
            .post(format!(
                "{API_ENDPOINT_SERVER_TO_TRUSTAUTH}/server/api/start-round"
            ))
            .json(&StartRoundRequest { muuid, name })
            .send()
            .await
            .and_then(|r| r.error_for_status())
            .map_err(|_| APIError::from_error_code(APIErrorCode::TrustAuthFetch))?
            .json::<StartRoundResponse>()
            .await
            .map_err(|_| APIError::from_error_code(APIErrorCode::TrustAuthFetch))?;

        BBSplusPublicKey::from_bytes(&resp.pub_key_bytes)
            .map_err(|_| APIError::from_error_code(APIErrorCode::TrustAuthFetch))
    }
}

pub fn init_state() -> anyhow::Result<AppState> {
    let is_secure = API_ENDPOINT.starts_with("https://");
    info!("Running rustsystem server with secure setting: {is_secure}");

    Ok(AppState(Arc::new(RwLock::new(AppStateInternal {
        secret: get_secret()?,
        meetings: Arc::new(AsyncRwLock::new(HashMap::new())),
        is_secure,
        trustauth_client: build_mtls_client(
            include_bytes!("../../mtls/ca/ca.crt"),
            include_bytes!("../../mtls/server/server.crt"),
            include_bytes!("../../mtls/server/server.key"),
        )?,
    }))))
}

pub fn app_public(state: AppState) -> Router {
    let serve_dir = ServeDir::new("frontend/dist")
        .not_found_service(ServeFile::new("frontend/dist/index.html"));

    Router::new()
        .fallback_service(serve_dir)
        .nest("/api", api_routes())
        .with_state(state)
}

pub fn app_internal(state: AppState) -> Router {
    Router::new()
        .nest("/trustauth", api_trustauth_routes())
        .with_state(state)
}
