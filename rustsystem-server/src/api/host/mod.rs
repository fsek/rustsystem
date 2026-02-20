use axum::{
    Router,
    routing::{delete, get, post},
};

use api_core::APIHandler;

use crate::AppState;

pub mod invite_watch;
use invite_watch::InviteWatch;

pub mod auth;

pub mod close_meeting;
use close_meeting::CloseMeeting;


pub mod state;
use state::{EndVoteRound, GetTally, StartVote, Tally};

pub mod new_voter;
use new_voter::{NewVoter, StartInvite};

pub mod user_management;
use user_management::{RemoveAll, RemoveVoter, ResetLogin, VoterId, VoterList};

// Routes at /api/host/...
pub fn host_routes() -> Router<AppState> {
    Router::new()
        .route("/start-vote", post(StartVote::handler))
        .route("/end-vote-round", delete(EndVoteRound::handler))
        .route("/tally", post(Tally::handler))
        .route("/get-tally", get(GetTally::handler))
        .route("/new-voter", post(NewVoter::handler))
        .route("/start-invite", post(StartInvite::handler))
        .route("/invite-watch", get(InviteWatch::handler))
        .route("/voter-list", get(VoterList::handler))
        .route("/voter-id", get(VoterId::handler))
        .route("/remove-all", delete(RemoveAll::handler))
        .route("/remove-voter", delete(RemoveVoter::handler))
        .route("/reset-login", post(ResetLogin::handler))
        .route("/close-meeting", delete(CloseMeeting::handler))
}
