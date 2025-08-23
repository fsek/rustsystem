use crate::AppState;
use axum::{Router, extract::FromRequest, http::StatusCode, response::IntoResponse, routing::post};

use api_core::APIHandler;

mod create_meeting;
use create_meeting::CreateMeeting;

mod login;
use login::Login;

mod auth;
use auth::AuthMeeting;

mod voter;
use voter::voter_routes;

mod host;
use host::host_routes;

mod common;
use common::common_routes;

// Routes at /api/...
pub fn api_routes() -> Router<AppState> {
    Router::new()
        .route("/create-meeting", post(CreateMeeting::handler))
        .route("/auth-meeting", post(AuthMeeting::handler))
        .route("/login", post(Login::handler))
        .nest("/host", host_routes())
        .nest("/voter", voter_routes())
        .nest("/common", common_routes())
}
