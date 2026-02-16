use axum::Router;
use invite_auth::InviteAuthority;
use std::{collections::HashMap, sync::Arc, time::SystemTime};
use tokens::{AuthUser, get_secret};
use tokio::sync::Mutex;
use tower_http::services::{ServeDir, ServeFile};
use tracing::info;

mod admin_auth;
pub mod api;
use api::api_routes;
pub mod vote_auth;
use vote_auth::VoteAuthority;
mod invite_auth;
pub mod tokens;
pub mod voting;

mod proof;

use uuid::Uuid;

use crate::admin_auth::AdminAuthority;

type MUuid = Uuid;
type UUuid = Uuid;

/// NOTE: The API_ENDPOINT environmental variable must be set at compile time!
const API_ENDPOINT: &str = env!("API_ENDPOINT");

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
    agenda: String,
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
pub struct AppState {
    secret: [u8; 32],
    meetings: ActiveMeetings,
    // decides whether cookies should be sent as secure (i.e. require https). This should be true
    // for prod and false for dev
    is_secure: bool,
}

pub fn app() -> Router {
    let is_secure = API_ENDPOINT.starts_with("https://");
    info!("Running rustsystem server with secure setting: {is_secure}");
    let state: AppState = AppState {
        secret: get_secret().unwrap(),
        meetings: Arc::new(Mutex::new(HashMap::new())),
        is_secure,
    };

    let serve_dir = ServeDir::new("frontend/dist")
        .not_found_service(ServeFile::new("frontend/dist/index.html"));

    Router::new()
        .fallback_service(serve_dir)
        .nest("/api", api_routes())
        .with_state(state)
}
