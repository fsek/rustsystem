use crate::AppState;
use axum::{
    Router,
    routing::{MethodRouter, Route, get, post},
};
mod create_meeting;
use create_meeting::create_meeting;

mod new_voter;
use new_voter::new_voter;

mod login;
use login::login;

mod auth;
use auth::auth_meeting;

pub mod vote;
use open_vote::{is_active, sse_watch_state, start_vote};
use vote::vote_api;

mod open_vote;

pub fn api_routes() -> Router<AppState> {
    Router::new()
        .route("/create-meeting", post(create_meeting))
        .route("/auth-meeting", post(auth_meeting))
        .route("/new-voter", post(new_voter))
        .route("/login", post(login))
        .route("/vote-active", get(is_active))
        .route("/start-vote", post(start_vote))
        .route("/events/vote-watch", get(sse_watch_state))
        .nest("/vote", vote_api())
}
