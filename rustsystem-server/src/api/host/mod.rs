use axum::{Router, routing::post};

use crate::AppState;

mod auth;

mod state;
use state::{start_vote, stop_vote};

mod new_voter;
use new_voter::new_voter;

// Routes at /api/host/...
pub fn host_routes() -> Router<AppState> {
    Router::new()
        .route("/start-vote", post(start_vote))
        .route("/stop-vote", post(stop_vote))
        .route("/new-voter", post(new_voter))
}
