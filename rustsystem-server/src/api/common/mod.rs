use axum::{Router, routing::get};

use crate::AppState;

mod state;
use state::is_active;

// Routes at /api/common/...
pub fn common_routes() -> Router<AppState> {
    Router::new().route("/vote-active", get(is_active))
}
