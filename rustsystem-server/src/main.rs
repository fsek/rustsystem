use axum::{Extension, Json, Router, http::StatusCode, response::IntoResponse, routing::post};
use axum_server::tls_rustls::RustlsConfig;
use rand::Rng;
use rustsystem_proof::{Provider, RegistrationResponse, Sha256Provider, ValidationInfo};
use std::{collections::HashMap, net::SocketAddr, sync::Arc, time::SystemTime};
use tokio::sync::Mutex;
use tower_http::services::{ServeDir, ServeFile};
use tracing::{error, info, level_filters::LevelFilter};
use tracing_subscriber::EnvFilter;
use zkryptium::{keys::pair::KeyPair, schemes::algorithms::BbsBls12381Sha256};

pub mod api;
use api::api_routes;
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
}
impl Meeting {
    pub fn add_voter(&mut self, uuid: UUID) -> Option<Voter> {
        self.voters.insert(uuid, Voter { logged_in: false })
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

    let keypair = Sha256Provider::generate_authentication_keys();
    let header = Header(b"Placeholder Header".to_vec());

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
        .route("/register", post(register))
        .route("/send-vote", post(validate_vote))
        .layer(Extension(Arc::new(AuthenticationKeys(keypair))))
        .layer(Extension(Arc::new(header)))
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

#[derive(Clone)]
pub struct AuthenticationKeys(KeyPair<BbsBls12381Sha256>);

#[derive(Clone)]
pub struct Header(Vec<u8>);

#[axum::debug_handler]
async fn register(
    Extension(keys): Extension<Arc<AuthenticationKeys>>,
    Extension(header): Extension<Arc<Header>>,
    Json(info_json): Json<serde_json::Value>,
) -> impl IntoResponse {
    info!("Got register request");
    let info = Sha256Provider::reg_info_from_json(info_json).unwrap();
    let signature =
        Sha256Provider::sign_token(info.commitment, header.0.clone(), keys.0.clone()).unwrap();

    let res = RegistrationResponse::Accepted(signature);

    (StatusCode::OK, Json(res))
}

#[axum::debug_handler]
async fn validate_vote(
    Extension(keys): Extension<Arc<AuthenticationKeys>>,
    Extension(header): Extension<Arc<Header>>,
    Json(info_json): Json<serde_json::Value>,
) -> impl IntoResponse {
    let info = Sha256Provider::val_info_from_json(info_json).unwrap();

    if let Ok(_) = Sha256Provider::validate_token(
        info.get_proof(),
        header.0.clone(),
        info.token,
        keys.0.public_key().clone(),
        info.signature,
    ) {
        info!("Validation Successful");
        (StatusCode::OK, Json("Success"))
    } else {
        error!("Validation Failure");
        (StatusCode::IM_A_TEAPOT, Json("Validation Failed"))
    }
}
