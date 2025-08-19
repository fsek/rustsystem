use axum::{
    Router,
    routing::{get, post},
};
use vote::{register, validate_vote};

use crate::AppState;

mod auth;

mod state;
use state::VoteWatch;

use super::APIHandler;

mod vote;

// Routes at /api/voter/...
pub fn voter_routes() -> Router<AppState> {
    Router::new()
        .route("/vote-watch", get(VoteWatch::handler))
        .route("/register", post(register))
        .route("/submit", post(validate_vote))
}
