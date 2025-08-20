use axum::{Router, routing::get};

use crate::AppState;

use super::APIHandler;

mod state;
use state::VoteActive;

pub mod common_responses;

// Routes at /api/common/...
pub fn common_routes() -> Router<AppState> {
    Router::new().route("/vote-active", get(VoteActive::handler))
}
