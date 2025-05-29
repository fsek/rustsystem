use axum::{
    Extension, Json, Router,
    http::StatusCode,
    response::{Html, IntoResponse},
    routing::{get, post},
};
use axum_server::tls_rustls::RustlsConfig;
use rustsystem_proof::{
    ProofContext, RegistrationInfo, authenticate_token_sha, generate_authentication_token_sha,
};
use serde::Deserialize;
use std::{error::Error, fs, net::SocketAddr, str::from_utf8, sync::Arc};
use tower_http::services::{ServeDir, ServeFile};
use tower_http::trace::TraceLayer;
use tracing_subscriber::{EnvFilter, fmt, layer::SubscriberExt, util::SubscriberInitExt};
use zkryptium::{
    keys::pair::KeyPair,
    schemes::{
        algorithms::{BBSplus, BbsBls12381Sha256},
        generics::{BlindSignature, Commitment},
    },
};

#[tokio::main]
async fn main() {
    let keypair = generate_authentication_token_sha();
    let header = Header(b"Placeholder Header".to_vec());

    fmt().with_env_filter(EnvFilter::from_default_env()).init();

    let app = Router::new()
        .route("/", get(index))
        .route("/try-post", post(test_post))
        .route("/register", post(register))
        .nest("/remote", rustsystem_remote::router())
        .nest_service(
            "/wrapper",
            ServeDir::new("../rustsystem-client/wrapper").append_index_html_on_directories(false),
        )
        .nest_service("/pkg", ServeDir::new("../rustsystem-client/pkg"))
        .layer(Extension(Arc::new(AuthenticationKeys(keypair))))
        .layer(Extension(Arc::new(header)))
        .layer(TraceLayer::new_for_http());

    let config = RustlsConfig::from_pem_file("certs/server.crt", "certs/server.key")
        .await
        .unwrap();

    let addr = SocketAddr::from(([127, 0, 0, 1], 8443));
    println!("listening on {}", addr);
    axum_server::bind_rustls(addr, config)
        .serve(app.into_make_service())
        .await
        .unwrap();
}

async fn index() -> Html<&'static str> {
    Html(include_str!("../../rustsystem-client/wrapper/index.html"))
}

#[derive(Clone)]
pub struct AuthenticationKeys(KeyPair<BbsBls12381Sha256>);

#[derive(Clone)]
pub struct Header(Vec<u8>);

async fn test_post(
    Extension(state): Extension<Arc<AuthenticationKeys>>,
    Json(info): Json<serde_json::Value>,
) -> impl IntoResponse {
    println!("Test works");
    println!("got {info}");
    (StatusCode::OK, Json("Got get!"))
}

#[axum::debug_handler]
async fn register(
    Extension(keys): Extension<Arc<AuthenticationKeys>>,
    Extension(header): Extension<Arc<Header>>,
    Json(info_json): Json<serde_json::Value>,
) -> impl IntoResponse {
    println!("Got this far!");
    let info = RegistrationInfo::<BbsBls12381Sha256>::deserialize(info_json).unwrap();
    let signature = authenticate_token_sha(
        info.context,
        info.commitment,
        header.0.clone(),
        keys.0.clone(),
    )
    .unwrap();

    let res = Json(serde_json::to_string(&signature).unwrap());
    println!("{res:?}");

    (StatusCode::OK, res)
}
