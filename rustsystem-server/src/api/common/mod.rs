use axum::Router;

use rustsystem_core::add_handler;

use crate::AppState;

mod meeting_specs;
use meeting_specs::MeetingSpecs;

mod meeting_specs_watch;
use meeting_specs_watch::MeetingSpecsWatch;

mod vote_active;
use vote_active::VoteActive;

mod vote_state_watch;
use vote_state_watch::VoteStateWatch;

mod vote_progress;
use vote_progress::VoteProgress;

mod vote_progress_watch;
use vote_progress_watch::VoteProgressWatch;

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
