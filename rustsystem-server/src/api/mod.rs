use axum::{
    Router,
    routing::{MethodRouter, Route, get, post},
};
use rustsystem_remote::router;

use crate::AppState;

mod create_meeting;
use create_meeting::create_meeting;

mod new_voter;
use new_voter::new_voter;

mod login;
use login::login;

mod auth;
use auth::auth_meeting;

pub fn api_routes() -> Router<AppState> {
    Router::new()
        .route("/create-meeting", post(create_meeting))
        .route("/auth-meeting", post(auth_meeting))
        .route("/new-voter", post(new_voter))
        .route("/login", post(login))
}
