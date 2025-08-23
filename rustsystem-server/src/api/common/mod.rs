use axum::{Router, routing::get};

use api_core::APIHandler;

use crate::AppState;

mod state;
use state::VoteActive;

mod meeting_specs;
use meeting_specs::MeetingSpecs;

pub mod common_responses;

// Routes at /api/common/...
pub fn common_routes() -> Router<AppState> {
    Router::new()
        .route("/vote-active", get(VoteActive::handler))
        .route("/meeting-specs", get(MeetingSpecs::handler))
}
