use axum::{
    Extension, Json, Router,
    extract::{Query, State},
    http::{Response, StatusCode, header},
    response::{Html, IntoResponse, Redirect},
    routing::{get, post},
};
use axum_extra::extract::{
    CookieJar,
    cookie::{self, Cookie},
};
use axum_server::tls_rustls::RustlsConfig;
use blake3::{Hash, Hasher, OUT_LEN, hash};
use rand::Rng;
use rustsystem_proof::{Provider, RegistrationResponse, Sha256Provider, ValidationInfo};
use serde::Deserialize;
use std::{
    collections::{HashMap, HashSet},
    net::SocketAddr,
    sync::Arc,
    time::SystemTime,
};
use tokio::sync::Mutex;
use tower_http::services::{ServeDir, ServeFile};
use tower_http::trace::TraceLayer;
use tracing_subscriber::{EnvFilter, fmt};
use uuid::Uuid;
use zkryptium::{keys::pair::KeyPair, schemes::algorithms::BbsBls12381Sha256};

use time::Duration;

pub mod session;
pub mod tokens;
pub mod voting;

use tokens::{AuthUser, generate_jwt, get_secret};

pub struct Voter {
    id: u128,
    logged_in: bool,
}

struct Meeting {
    creator: String,
    title: String,
    start_time: SystemTime,
    users: HashMap<u128, Voter>,
}

pub type ActiveMeetings = Arc<Mutex<HashMap<String, Meeting>>>;
pub type TokenStore = Arc<Mutex<HashMap<String, String>>>;

#[derive(Clone)]
pub struct AppState {
    secret: [u8; 32],
    meetings: ActiveMeetings,
}

#[tokio::main]
async fn main() {
    // fmt().with_env_filter(EnvFilter::from_default_env()).init();
    //
    // // TEMhttp://localhost:3000/create-meetingPORARY FOR TESTING AND DEMOSTRATION OF SETUP
    // //session::gen_qr_code().unwrap();
    //
    let keypair = Sha256Provider::generate_authentication_keys();
    let header = Header(b"Placeholder Header".to_vec());

    let state: AppState = AppState {
        secret: get_secret().unwrap(),
        meetings: Arc::new(Mutex::new(HashMap::new())),
    };

    let user_id = u128::from_be_bytes(rand::random()); // This should be a randomly generated hash later on!
    let user = Voter {
        id: user_id.clone(),
        logged_in: false,
    };
    let mut users = HashMap::new();
    users.insert(user_id, user);

    let serve_dir = ServeDir::new("../rustsystem-client/static")
        .not_found_service(ServeFile::new("../rustsystem-client/static/index.html"));

    let app = Router::new()
        .fallback_service(serve_dir)
        .route("/create-meeting", post(create_meeting))
        .route("/register", post(register))
        .route("/send-vote", post(validate_vote))
        .route("/protected", get(protected))
        .layer(Extension(Arc::new(AuthenticationKeys(keypair))))
        .layer(Extension(Arc::new(header)))
        .with_state(state);
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
    pub cred: u128,
    pub meeting: String,
}

// Cookies expire after 10 hours
const COOKIE_LIFETIME: Duration = Duration::hours(10);

#[derive(Deserialize)]
struct CreateMeeting {
    pub user_name: String,
    pub meeting_name: String,
}

fn gen_token() -> String {
    let mut bytes = [0u8; 32]; // 256-bit token
    rand::rng().fill(&mut bytes);
    hex::encode(bytes)
}

#[axum::debug_handler]
async fn create_meeting(
    jar: CookieJar,
    State(state): State<AppState>,
    Json(body): Json<CreateMeeting>,
) -> impl IntoResponse {
    let jwt = generate_jwt(&hash(body.user_name.as_bytes()).to_hex(), &state.secret);
    let new_cookie = Cookie::build(("access_token", jwt))
        .http_only(true)
        .secure(true)
        .same_site(cookie::SameSite::Strict)
        .path("/");

    (
        StatusCode::CREATED,
        jar.add(new_cookie),
        Json(format!(
            "user {} successfully authenticated",
            body.user_name
        )),
    )
}

async fn protected(
    AuthUser { user_id }: AuthUser,
    State(state): State<AppState>,
) -> impl IntoResponse {
    Json(format!("Hello user with ID: {user_id}"))
}

async fn voter_login(
    jar: CookieJar,
    cred: Query<LoginCredentials>,
    State(state): State<AppState>,
) -> impl IntoResponse {
    let user_id = cred.0.cred;
    let meeting_hash = cred.0.meeting;

    if let Some(meeting) = state.meetings.lock().await.get_mut(&meeting_hash) {
        if let Some(voter) = meeting.users.get_mut(&user_id) {
            if voter.logged_in {
                return (
                    StatusCode::UNAUTHORIZED,
                    Json(format!("Voter with id {user_id} has already logged in.")),
                )
                    .into_response();
            } else {
                voter.logged_in = true;

                let session_id = Uuid::new_v4().to_string();

                // This should return an auth cookie and a redirect to the main meeting page
                return (StatusCode::OK, Json("Logged In!!!")).into_response();
            }
        } else {
            (
                StatusCode::UNAUTHORIZED,
                Json(format!(
                    "Supplied user id {user_id} not found in meeting {meeting_hash}"
                )),
            )
                .into_response()
        }
    } else {
        (
            StatusCode::UNAUTHORIZED,
            Json(format!("Meeting {meeting_hash} does not exist")),
        )
            .into_response()
    }
}

#[derive(Deserialize)]
struct MeetingInfo {
    hash: String,
}

// async fn serve_voter_page(
//     jar: CookieJar,
//     State(state): State<AppState>,
//     cred: Query<MeetingInfo>,
// ) -> impl IntoResponse {
//     if let Some(session_cookie) = jar.get("session_id") {
//         let session_id = session_cookie.value();
//         if let Some(user_id) = state.user_tokens.lock().await.get(session_id) {
//             format!("Welcome back, {}!", user_id).into_response()
//         } else {
//             (StatusCode::UNAUTHORIZED).into_response()
//         }
//     } else {
//         (StatusCode::UNAUTHORIZED).into_response()
//     }
// }

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
