use std::net::SocketAddr;
use tracing::{info, level_filters::LevelFilter};

use tracing_subscriber::EnvFilter;

use rustsystem_server::app;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env().add_directive(LevelFilter::INFO.into()))
        .init();

    let app = app();

    let addr = SocketAddr::from(([0, 0, 0, 0], 3000));
    info!("Running server on {addr}");
    axum_server::bind(addr)
        .serve(app.into_make_service())
        .await
        .unwrap();
}
