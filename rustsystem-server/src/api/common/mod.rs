use axum::{
    Router,
    routing::{get, post},
};

use api_core::APIHandler;

use crate::AppState;

mod state;
use state::{VoteActive, VoteStateWatch};

mod meeting_specs;
use meeting_specs::{MeetingSpecs, MeetingSpecsWatch, UpdateAgenda};

pub mod common_responses;

// Routes at /api/common/...
pub fn common_routes() -> Router<AppState> {
    Router::new()
        .route("/vote-state-watch", get(VoteStateWatch::handler))
        .route("/vote-active", get(VoteActive::handler))
        .route("/meeting-specs", get(MeetingSpecs::handler))
        .route("/meeting-specs-watch", get(MeetingSpecsWatch::handler))
        .route("/update-agenda", post(UpdateAgenda::handler))
}
