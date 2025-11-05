use tokio::net::TcpListener;
use tracing::{info, level_filters::LevelFilter};
use tracing_subscriber::EnvFilter;

use app_state::AppState;

mod app_state;
mod router;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::builder()
                .with_default_directive(LevelFilter::INFO.into())
                .from_env_lossy(),
        )
        .init();

    let listener = TcpListener::bind("[::]:8000").await.unwrap();

    info!("Binding {:?}", listener.local_addr().unwrap());

    let state = app_state::AppState::new("http://yoga:8000".parse().unwrap());

    axum::serve(
        listener,
        router::router().with_state(state).into_make_service(),
    )
    .await
    .unwrap();
}
