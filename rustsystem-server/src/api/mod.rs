use crate::AppState;
use axum::Router;

use rustsystem_core::add_handler;

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
    let mut router = Router::new();
    router = add_handler::<CreateMeeting>(router);
    router = add_handler::<SessionIds>(router);
    router = add_handler::<Login>(router);
    router
        .nest("/host", host_routes())
        .nest("/voter", voter_routes())
        .nest("/common", common_routes())
}
