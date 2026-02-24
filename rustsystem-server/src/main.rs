use rustsystem_core::mtls::build_mtls_server_config;
use axum_server::tls_rustls::RustlsConfig;
use futures::FutureExt;
use std::{net::SocketAddr, sync::Arc};
use tracing::{info, level_filters::LevelFilter};
use tracing_subscriber::EnvFilter;

use rustsystem_server::{app_internal, app_public, init_state};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env().add_directive(LevelFilter::INFO.into()))
        .init();

    let state = init_state().unwrap();

    let app_public = app_public(state.clone());
    let app_internal = app_internal(state);

    let tls_config = build_mtls_server_config(
        include_bytes!("../../mtls/server/server.crt"),
        include_bytes!("../../mtls/server/server.key"),
        include_bytes!("../../mtls/ca/ca.crt"),
    )
    .unwrap();

    let addr_public = SocketAddr::from(([0, 0, 0, 0], 1443));
    let addr_internal = SocketAddr::from(([0, 0, 0, 0], 1444));

    let internal_serve = axum_server::bind_rustls(
        addr_internal,
        RustlsConfig::from_config(std::sync::Arc::new(tls_config)),
    )
    .serve(app_internal.into_make_service());

    let public_serve = axum_server::bind(addr_public).serve(app_public.into_make_service());

    let (internal_res, public_res) = tokio::try_join!(internal_serve, public_serve)?;
    Ok(())
}
