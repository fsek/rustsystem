use axum::{Extension, Json, Router, http::StatusCode, response::IntoResponse, routing::post};
use axum_server::tls_rustls::RustlsConfig;
use rand::Rng;
use rustsystem_proof::{Provider, RegistrationResponse, Sha256Provider, ValidationInfo};
use std::{
    collections::{HashMap, HashSet},
    net::SocketAddr,
    sync::Arc,
    time::SystemTime,
};
use tokio::sync::Mutex;
use tower_http::services::{ServeDir, ServeFile};
use tracing::{error, info, level_filters::LevelFilter};
use tracing_subscriber::EnvFilter;
use zkryptium::{keys::pair::KeyPair, schemes::algorithms::BbsBls12381Sha256};

pub mod api;
use api::{api_routes, vote::VoteAuthority};
pub mod tokens;
pub mod voting;

use tokens::{AuthUser, get_secret};

/// NOTE: The API_ENDPOINT environmental variable must be set at compile time!
const API_ENDPOINT: &str = env!("API_ENDPOINT");

pub fn rand_u128() -> u128 {
    let mut res = [0u8; 16];
    rand::rng().fill(&mut res);
    u128::from_be_bytes(res)
}
type UUID = u128;
pub fn new_uuid() -> UUID {
    rand_u128()
}
type MUID = u128;
pub fn new_muid() -> MUID {
    rand_u128()
}

#[derive(Debug)]
pub struct Voter {
    logged_in: bool,
}

pub struct Meeting {
    host: UUID,
    title: String,
    start_time: SystemTime,
    voters: HashMap<u128, Voter>,
    vote_auth: VoteAuthority,
}
impl Meeting {
    pub fn add_voter(&mut self, uuid: UUID) -> Option<Voter> {
        self.voters.insert(uuid, Voter { logged_in: false })
    }

    pub fn get_auth(&mut self) -> &mut VoteAuthority {
        &mut self.vote_auth
    }
}

pub type ActiveMeetings = Arc<Mutex<HashMap<MUID, Meeting>>>;

#[derive(Clone)]
pub struct AppState {
    secret: [u8; 32],
    meetings: ActiveMeetings,
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env().add_directive(LevelFilter::INFO.into()))
        .init();

    let state: AppState = AppState {
        secret: get_secret().unwrap(),
        meetings: Arc::new(Mutex::new(HashMap::new())),
    };

    let user_id = u128::from_be_bytes(rand::random()); // This should be a randomly generated hash later on!
    let user = Voter { logged_in: false };
    let mut users = HashMap::new();
    users.insert(user_id, user);

    let serve_dir = ServeDir::new("../rustsystem-client/static")
        .not_found_service(ServeFile::new("../rustsystem-client/static/index.html"));

    let app = Router::new()
        .fallback_service(serve_dir)
        .nest("/api", api_routes())
        .with_state(state);

    let config = RustlsConfig::from_pem_file("localhost+1.pem", "localhost+1-key.pem")
        .await
        .unwrap();

    let addr = SocketAddr::from(([0, 0, 0, 0], 3000));
    info!("Running server on {addr}");
    axum_server::bind_rustls(addr, config)
        .serve(app.into_make_service())
        .await
        .unwrap();
}
