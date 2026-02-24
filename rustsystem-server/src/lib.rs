use rustsystem_core::{APIError, APIErrorCode, mtls::build_mtls_client};
use axum::Router;
use invite_auth::InviteAuthority;
use reqwest::Client;
use serde::Deserialize;
use std::{
    collections::HashMap,
    sync::{Arc, RwLock, RwLockReadGuard, RwLockWriteGuard},
    time::SystemTime,
};
use tokens::{AuthUser, get_secret};
use tokio::sync::Mutex;
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
    registered_at: std::time::SystemTime,
}

pub struct Meeting {
    title: String,
    start_time: SystemTime,
    voters: HashMap<Uuid, Voter>,
    vote_auth: VoteAuthority,
    invite_auth: InviteAuthority,
    admin_auth: AdminAuthority,
    locked: bool,
}
impl Meeting {
    pub fn add_voter(&mut self, name: String, uuid: UUuid, is_host: bool) -> Option<Voter> {
        self.voters.insert(
            uuid,
            Voter {
                name,
                logged_in: false,
                is_host,
                registered_at: std::time::SystemTime::now(),
            },
        )
    }

    pub fn has_voter_with_name(&self, name: &String) -> bool {
        self.voters.iter().any(|(_id, v)| &v.name == name)
    }

    pub fn get_auth(&mut self) -> &mut VoteAuthority {
        &mut self.vote_auth
    }

    pub fn get_start_time(&self) -> SystemTime {
        self.start_time
    }

    pub fn remove_unclaimed_voters(&mut self) {
        self.voters.retain(|_id, voter| voter.logged_in);
    }

    // Locking also removes unclaimed voters
    pub fn lock(&mut self) {
        self.remove_unclaimed_voters();
        self.locked = true;
    }

    pub fn unlock(&mut self) {
        self.locked = false;
    }
}

pub type ActiveMeetings = Arc<Mutex<HashMap<MUuid, Meeting>>>;

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

    pub fn write(&self) -> Result<RwLockWriteGuard<'_, AppStateInternal>, APIError> {
        self.0
            .write()
            .map_err(|_e| APIError::from_error_code(APIErrorCode::StateCurrupt))
    }

    pub fn meetings(&self) -> Result<ActiveMeetings, APIError> {
        let guard = self.read()?;
        Ok(guard.meetings.clone())
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
        meetings: Arc::new(Mutex::new(HashMap::new())),
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
