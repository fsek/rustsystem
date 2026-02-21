use axum::{Router, routing::{get, post}};
use vote::{Register, Submit};
use status::{IsRegistered, IsSubmitted};

use api_core::APIHandler;

use crate::AppState;

mod vote;
mod status;

// Routes at /api/voter/...
pub fn voter_routes() -> Router<AppState> {
    Router::new()
        .route("/register", post(Register::handler))
        .route("/submit", post(Submit::handler))
        .route("/is-registered", get(IsRegistered::handler))
        .route("/is-submitted", post(IsSubmitted::handler))
}
