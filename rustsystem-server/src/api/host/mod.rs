use axum::{
    Router,
    routing::{get, post},
};
use invite_event::sse_watch_invite;

use crate::AppState;

mod auth;

mod state;
use state::{start_vote, tally};

mod new_voter;
use new_voter::{new_voter, start_invite};

mod invite_event;

// Routes at /api/host/...
pub fn host_routes() -> Router<AppState> {
    Router::new()
        .route("/start-vote", post(start_vote))
        .route("/tally", post(tally))
        .route("/new-voter", post(new_voter))
        .route("/start-invite", post(start_invite))
        .route("/invite-watch", get(sse_watch_invite))
}
