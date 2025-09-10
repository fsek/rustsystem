use axum::{
    Router,
    routing::{get, post},
};
use vote::{Register, Submit};

use api_core::APIHandler;

use crate::AppState;

mod auth;
mod vote;

// Routes at /api/voter/...
pub fn voter_routes() -> Router<AppState> {
    Router::new()
        .route("/register", post(Register::handler))
        .route("/submit", post(Submit::handler))
}
