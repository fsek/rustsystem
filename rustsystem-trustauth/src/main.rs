use std::{net::SocketAddr, process::ExitCode, sync::Arc};

use axum_server::tls_rustls::RustlsConfig;
use tracing::{error, info};

use rustsystem_core::{
    limits::{GENERAL, rate_limit},
    logging::init_logging,
    mtls::build_mtls_server_config,
};
use rustsystem_trustauth::{AppState, config::Config, internal_router, public_router};

#[tokio::main]
async fn main() -> ExitCode {
    // Held until exit so buffered log lines are flushed.
    let _guard = init_logging("trustauth.log");

    let config = match Config::from_env() {
        Ok(config) => config,
        Err(e) => {
            error!("{e}");
            return ExitCode::FAILURE;
        }
    };
    let tls = match build_mtls_server_config(&config.cert, &config.key, &config.ca_cert) {
        Ok(tls) => tls,
        Err(e) => {
            error!("{e}");
            return ExitCode::FAILURE;
        }
    };

    let app = AppState::new(config.secure_cookies);
    app.spawn_pruning();

    let public = public_router(
        app.clone(),
        config.allowed_origins,
        rate_limit(GENERAL, config.client_ips),
    );
    let internal = internal_router(app);

    info!(public = %config.public_addr, internal = %config.internal_addr, "Trustauth listening");

    let public_serve = axum_server::bind(config.public_addr)
        .serve(public.into_make_service_with_connect_info::<SocketAddr>());
    let internal_serve = axum_server::bind_rustls(config.internal_addr, RustlsConfig::from_config(Arc::new(tls)))
        .serve(internal.into_make_service());

    match tokio::try_join!(public_serve, internal_serve) {
        Ok(_) => ExitCode::SUCCESS,
        Err(e) => {
            error!("listener failed: {e}");
            ExitCode::FAILURE
        }
    }
}
