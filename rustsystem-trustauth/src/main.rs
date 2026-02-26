use rustsystem_core::{APIError, APIErrorCode, mtls::build_mtls_server_config};
use axum_server::tls_rustls::RustlsConfig;
use std::{net::SocketAddr, sync::Arc};
use tracing::{info, level_filters::LevelFilter};
use tracing_subscriber::EnvFilter;

use rustsystem_trustauth::{app_internal, app_public, init_state};

#[tokio::main]
async fn main() -> Result<(), APIError> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env().add_directive(LevelFilter::INFO.into()))
        .init();

    let state = init_state()?;

    let app_public = app_public(state.clone())?;
    let app_internal = app_internal(state);

    let addr_public = SocketAddr::from(([0, 0, 0, 0], 2443));
    let addr_internal = SocketAddr::from(([0, 0, 0, 0], 2444));

    let tls_config = build_mtls_server_config(
        include_bytes!("../../mtls/trustauth/trustauth.crt"),
        include_bytes!("../../mtls/trustauth/trustauth.key"),
        include_bytes!("../../mtls/ca/ca.crt"),
    )?;

    info!("Running trustauth server on {addr_public}");
    let public_serve = axum_server::bind(addr_public).serve(app_public.into_make_service());
    let internal_serve = axum_server::bind_rustls(
        addr_internal,
        RustlsConfig::from_config(Arc::new(tls_config)),
    )
    .serve(app_internal.into_make_service());

    tokio::try_join!(internal_serve, public_serve)
        .map_err(|_| APIError::from_error_code(APIErrorCode::InitError))?;
    Ok(())
}
