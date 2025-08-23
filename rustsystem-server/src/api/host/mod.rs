use axum::{
    Router,
    routing::{get, post},
};

use api_core::APIHandler;

use crate::AppState;

mod invite_event;
use invite_event::InviteWatch;

mod auth;

mod state;
use state::{StartVote, Tally};

mod new_voter;
use new_voter::{NewVoter, StartInvite};

// Routes at /api/host/...
pub fn host_routes() -> Router<AppState> {
    Router::new()
        .route("/start-vote", post(StartVote::handler))
        .route("/tally", post(Tally::handler))
        .route("/new-voter", post(NewVoter::handler))
        .route("/start-invite", post(StartInvite::handler))
        .route("/invite-watch", get(InviteWatch::handler))
}
