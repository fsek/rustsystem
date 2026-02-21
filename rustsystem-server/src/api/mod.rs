use crate::AppState;
use axum::{
    Router,
    routing::{get, post},
};

use api_core::APIHandler;

pub mod create_meeting;
use create_meeting::CreateMeeting;

pub mod login;
use login::Login;

pub mod session_ids;
use session_ids::SessionIds;

pub mod voter;
use voter::voter_routes;

pub mod host;
use host::host_routes;

pub mod common;
use common::common_routes;

// Routes at /api/...
pub fn api_routes() -> Router<AppState> {
    Router::new()
        .route("/create-meeting", post(CreateMeeting::handler))
        .route("/session-ids", get(SessionIds::handler))
        .route("/login", post(Login::handler))
        .nest("/host", host_routes())
        .nest("/voter", voter_routes())
        .nest("/common", common_routes())
}
