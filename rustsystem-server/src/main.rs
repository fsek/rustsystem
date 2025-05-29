use axum::{
    Extension, Json, Router,
    http::StatusCode,
    response::{Html, IntoResponse},
    routing::{get, post},
};
use axum_server::tls_rustls::RustlsConfig;
use rustsystem_proof::{Provider, RegistrationResponse, Sha256Provider, ValidationInfo};
use std::{net::SocketAddr, sync::Arc};
use tower_http::services::ServeDir;
use tower_http::trace::TraceLayer;
use tracing_subscriber::{EnvFilter, fmt};
use zkryptium::{keys::pair::KeyPair, schemes::algorithms::BbsBls12381Sha256};

#[tokio::main]
async fn main() {
    let keypair = Sha256Provider::generate_authentication_keys();
    let header = Header(b"Placeholder Header".to_vec());

    fmt().with_env_filter(EnvFilter::from_default_env()).init();

    let app = Router::new()
        .route("/", get(index))
        .route("/register", post(register))
        .route("/vote", post(validate_vote))
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

#[axum::debug_handler]
async fn register(
    Extension(keys): Extension<Arc<AuthenticationKeys>>,
    Extension(header): Extension<Arc<Header>>,
    Json(info_json): Json<serde_json::Value>,
) -> impl IntoResponse {
    let info = Sha256Provider::reg_info_from_json(info_json).unwrap();
    let signature =
        Sha256Provider::sign_token(info.commitment, header.0.clone(), keys.0.clone()).unwrap();

    let res = RegistrationResponse::Accepted(signature);
    println!("{res:?}");

    (StatusCode::OK, Json(serde_json::to_string(&res).unwrap()))
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
