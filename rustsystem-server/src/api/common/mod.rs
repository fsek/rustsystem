use axum::Router;

use rustsystem_core::add_handler;

use crate::AppState;

mod state;
use state::{VoteActive, VoteProgress, VoteProgressWatch, VoteStateWatch};

mod meeting_specs;
use meeting_specs::{MeetingSpecs, MeetingSpecsWatch, UpdateAgenda};

pub mod common_responses;

// Routes at /api/common/...
pub fn common_routes() -> Router<AppState> {
    let mut router = Router::new();
    router = add_handler::<MeetingSpecs>(router);
    router = add_handler::<MeetingSpecsWatch>(router);
    router = add_handler::<VoteActive>(router);
    router = add_handler::<VoteStateWatch>(router);
    router = add_handler::<VoteProgress>(router);
    router = add_handler::<VoteProgressWatch>(router);
    router
}
