use axum::Router;

use rustsystem_core::add_handler;

use crate::AppState;

pub mod invite_watch;
use invite_watch::InviteWatch;

pub mod auth;

pub mod close_meeting;
use close_meeting::CloseMeeting;

pub mod start_vote;
use start_vote::StartVote;

pub mod tally;
use tally::Tally;

pub mod get_tally;
use get_tally::GetTally;

pub mod get_all_tally;
use get_all_tally::GetAllTally;

pub mod end_vote_round;
use end_vote_round::EndVoteRound;

pub mod new_voter;
use new_voter::NewVoter;

pub mod start_invite;
use start_invite::StartInvite;

pub mod voter_list;
use voter_list::VoterList;

pub mod voter_id;
use voter_id::VoterId;

pub mod remove_all;
use remove_all::RemoveAll;

pub mod remove_voter;
use remove_voter::RemoveVoter;

pub mod reset_login;
use reset_login::ResetLogin;

// Routes at /api/host/...
pub fn host_routes() -> Router<AppState> {
    let mut router = Router::new();
    router = add_handler::<StartVote>(router);
    router = add_handler::<EndVoteRound>(router);
    router = add_handler::<Tally>(router);
    router = add_handler::<GetTally>(router);
    router = add_handler::<GetAllTally>(router);
    router = add_handler::<NewVoter>(router);
    router = add_handler::<StartInvite>(router);
    router = add_handler::<InviteWatch>(router);
    router = add_handler::<VoterList>(router);
    router = add_handler::<VoterId>(router);
    router = add_handler::<RemoveAll>(router);
    router = add_handler::<RemoveVoter>(router);
    router = add_handler::<ResetLogin>(router);
    router = add_handler::<CloseMeeting>(router);
    router
}
