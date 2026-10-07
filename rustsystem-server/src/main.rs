use std::{net::SocketAddr, process::ExitCode};

use tracing::{error, info};

use rustsystem_core::{
    limits::{CREATE_MEETING, GENERAL, rate_limit},
    logging::init_logging,
};
use rustsystem_server::{AppState, RateLimits, config::Config, router};

#[tokio::main]
async fn main() -> ExitCode {
    // Held until exit so buffered log lines are flushed.
    let _guard = init_logging("server.log");

    let config = match Config::from_env() {
        Ok(config) => config,
        Err(e) => {
            error!("{e}");
            return ExitCode::FAILURE;
        }
    };

    let app = AppState::new(config.settings, config.trustauth);
    app.spawn_pruning();

    let limits = RateLimits {
        general: rate_limit(GENERAL, config.client_ips.clone()),
        create_meeting: rate_limit(CREATE_MEETING, config.client_ips),
    };
    let service = router(app, &config.frontend_dir, limits);

    info!(addr = %config.addr, "Server listening");
    let result = axum_server::bind(config.addr)
        .serve(service.into_make_service_with_connect_info::<SocketAddr>())
        .await;
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            error!("listener failed: {e}");
            ExitCode::FAILURE
        }
    }
}
