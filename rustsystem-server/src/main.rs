use axum::{
    Extension, Json, Router,
    extract::{Query, State},
    http::StatusCode,
    response::{Html, IntoResponse, Redirect},
    routing::{get, post},
};
use axum_extra::extract::{CookieJar, cookie::Cookie};
use axum_server::tls_rustls::RustlsConfig;
use blake3::OUT_LEN;
use rustsystem_proof::{Provider, RegistrationResponse, Sha256Provider, ValidationInfo};
use rustsystem_server::session;
use serde::Deserialize;
use std::{
    collections::{HashMap, HashSet},
    net::SocketAddr,
    sync::Arc,
};
use tokio::sync::Mutex;
use tower_http::services::{ServeDir, ServeFile};
use tower_http::trace::TraceLayer;
use tracing_subscriber::{EnvFilter, fmt};
use uuid::Uuid;
use zkryptium::{keys::pair::KeyPair, schemes::algorithms::BbsBls12381Sha256};

use time::Duration;

#[tokio::main]
async fn main() {
    // fmt().with_env_filter(EnvFilter::from_default_env()).init();
    //
    // // TEMPORARY FOR TESTING AND DEMOSTRATION OF SETUP
    // //session::gen_qr_code().unwrap();
    //
    let keypair = Sha256Provider::generate_authentication_keys();
    let header = Header(b"Placeholder Header".to_vec());
    //
    // let state: AppState = AppState {
    //     meetings: Arc::new(Mutex::new(HashMap::new())),
    //     sessions: Arc::new(Mutex::new(HashMap::new())),
    // };
    //
    // let user_id = String::from("TestUser"); // This should be a randomly generated hash later on!
    // let user = User {
    //     id: user_id.clone(),
    //     logged_in: false,
    // };
    // let mut users = HashMap::new();
    // users.insert(user_id, user);
    // state
    //     .meetings
    //     .lock()
    //     .await
    //     .insert(String::from("TestMeeting"), users);
    // // -----------------------------------------------

    let serve_dir = ServeDir::new("../rustsystem-client/static")
        .not_found_service(ServeFile::new("../rustsystem-client/static/index.html"));
    let app = Router::new()
        .fallback_service(serve_dir)
        .route("/send-vote", post(validate_vote))
        .route("/register", post(register))
        .layer(Extension(Arc::new(AuthenticationKeys(keypair))))
        .layer(Extension(Arc::new(header)));
    // let app = Router::new()
    //     .fallback_service(serve_dir)
    //     // .route("/", get(index))
    //     // .route("/login", get(voter_login))
    //     // .route("/in-meeting", get(serve_voter_page))
    //     // .merge(rustsystem_remote::router())
    //     // .nest_service(
    //     //     "/wrapper",
    //     //     ServeDir::new("../rustsystem-client/wrapper").append_index_html_on_directories(false),
    //     // )
    //     // .nest_service("/pkg", ServeDir::new("../rustsystem-client/pkg"))

    //     // .layer(TraceLayer::new_for_http())
    //     .with_state(state);

    let addr = SocketAddr::from(([0, 0, 0, 0], 3000));
    println!("listening on {}", addr);
    axum_server::bind(addr)
        .serve(app.into_make_service())
        .await
        .unwrap();
}

#[derive(Clone)]
pub struct AuthenticationKeys(KeyPair<BbsBls12381Sha256>);

#[derive(Clone)]
pub struct Header(Vec<u8>);

#[derive(Deserialize)]
struct LoginCredentials {
    pub cred: String,
    pub meeting: String,
}

struct User {
    id: String,
    logged_in: bool,
}

type Users = HashMap<String, User>;
type ActiveMeetings = Arc<Mutex<HashMap<String, Users>>>;
type SessionStore = Arc<Mutex<HashMap<String, String>>>;

#[derive(Clone)]
struct AppState {
    meetings: ActiveMeetings,
    sessions: SessionStore,
}

// Cookies expire after 10 hours
const COOKIE_LIFETIME: Duration = Duration::hours(10);

async fn voter_login(
    jar: CookieJar,
    cred: Query<LoginCredentials>,
    State(state): State<AppState>,
) -> impl IntoResponse {
    let user_id = cred.0.cred;
    let meeting_hash = cred.0.meeting;

    if let Some(meeting_users) = state.meetings.lock().await.get_mut(&meeting_hash) {
        if let Some(user) = meeting_users.get_mut(&user_id) {
            if user.logged_in {
                return (
                    StatusCode::UNAUTHORIZED,
                    format!("User with id {user_id} has already logged in."),
                )
                    .into_response();
            } else {
                user.logged_in = true;

                let session_id = Uuid::new_v4().to_string();

                state
                    .sessions
                    .lock()
                    .await
                    .insert(session_id.clone(), user_id);
                let cookie = Cookie::build(("session_id", session_id))
                    .path("/in-meeting")
                    .http_only(true)
                    .secure(true)
                    .max_age(COOKIE_LIFETIME);

                (
                    jar.add(cookie),
                    Redirect::to(&format!("/in-meeting?hash={meeting_hash}")),
                )
                    .into_response()
            }
        } else {
            (
                StatusCode::UNAUTHORIZED,
                format!("Supplied user id {user_id} not found in meeting {meeting_hash}"),
            )
                .into_response()
        }
    } else {
        (
            StatusCode::UNAUTHORIZED,
            format!("Meeting {meeting_hash} does not exist"),
        )
            .into_response()
    }
}

#[derive(Deserialize)]
struct MeetingInfo {
    hash: String,
}

async fn serve_voter_page(
    jar: CookieJar,
    State(state): State<AppState>,
    cred: Query<MeetingInfo>,
) -> impl IntoResponse {
    if let Some(session_cookie) = jar.get("session_id") {
        let session_id = session_cookie.value();
        if let Some(user_id) = state.sessions.lock().await.get(session_id) {
            format!("Welcome back, {}!", user_id).into_response()
        } else {
            (StatusCode::UNAUTHORIZED).into_response()
        }
    } else {
        (StatusCode::UNAUTHORIZED).into_response()
    }
}

#[axum::debug_handler]
async fn register(
    Extension(keys): Extension<Arc<AuthenticationKeys>>,
    Extension(header): Extension<Arc<Header>>,
    Json(info_json): Json<serde_json::Value>,
) -> impl IntoResponse {
    println!("Got register request");
    let info = Sha256Provider::reg_info_from_json(info_json).unwrap();
    let signature =
        Sha256Provider::sign_token(info.commitment, header.0.clone(), keys.0.clone()).unwrap();

    let res = RegistrationResponse::Accepted(signature);
    println!("{res:?}");

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
        println!("Validation Successful");
        (StatusCode::OK, Json("Success"))
    } else {
        println!("Validation Failure");
        (StatusCode::IM_A_TEAPOT, Json("Validation Failed"))
    }
}
