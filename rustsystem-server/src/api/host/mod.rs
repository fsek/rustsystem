use axum::{Router, routing::post};

use crate::AppState;

mod state;
use state::{start_vote, stop_vote};

// Routes at /api/host/...
pub fn host_routes() -> Router<AppState> {
    Router::new()
        .route("/start-vote", post(start_vote))
        .route("/stop-vote", post(stop_vote))
}
