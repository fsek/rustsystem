use axum::{
    Router,
    routing::{delete, get, post},
};

use api_core::APIHandler;

use crate::{
    AppState,
    api::host::{
        state::{EndVoteRound, Lock, Unlock},
        user_management::{RemoveVoter, VoterId, VoterList},
    },
};

pub mod invite_watch;
use invite_watch::InviteWatch;

pub mod auth;

pub mod state;
use state::{StartVote, Tally};

pub mod new_voter;
use new_voter::{NewVoter, StartInvite};

pub mod user_management;

// Routes at /api/host/...
pub fn host_routes() -> Router<AppState> {
    Router::new()
        .route("/lock", post(Lock::handler))
        .route("/unlock", post(Unlock::handler))
        .route("/start-vote", post(StartVote::handler))
        .route("/end-vote-round", delete(EndVoteRound::handler))
        .route("/tally", get(Tally::handler))
        .route("/new-voter", post(NewVoter::handler))
        .route("/start-invite", post(StartInvite::handler))
        .route("/invite-watch", get(InviteWatch::handler))
        .route("/voter-list", get(VoterList::handler))
        .route("/voter-id", get(VoterId::handler))
        .route("/remove-voter", delete(RemoveVoter::handler))
}
