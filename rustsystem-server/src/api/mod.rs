use crate::AppState;
use axum::{
    Router,
    routing::{get, post},
};
mod create_meeting;
use create_meeting::create_meeting;

mod new_voter;
use new_voter::new_voter;

mod login;
use login::login;

mod auth;
use auth::auth_meeting;

mod voter;
use voter::voter_routes;

mod host;
use host::host_routes;

mod common;
use common::common_routes;

// Routes at /api/...
pub fn api_routes() -> Router<AppState> {
    Router::new()
        .route("/create-meeting", post(create_meeting))
        .route("/auth-meeting", post(auth_meeting))
        .route("/new-voter", post(new_voter))
        .route("/login", post(login))
        .nest("/host", host_routes())
        .nest("/voter", voter_routes())
        .nest("/common", common_routes())
}
