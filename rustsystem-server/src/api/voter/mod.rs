use axum::{Router, routing::post};
use vote::{Register, Submit};

use api_core::APIHandler;

use crate::AppState;

mod vote;

// Routes at /api/voter/...
pub fn voter_routes() -> Router<AppState> {
    Router::new()
        .route("/register", post(Register::handler))
        .route("/submit", post(Submit::handler))
}
