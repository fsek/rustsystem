use rustsystem_core::add_handler;
use axum::Router;

use crate::AppState;

mod vote_active;
use vote_active::VoteActive;

mod is_voter;
use is_voter::IsVoter;

pub fn api_trustauth_routes() -> Router<AppState> {
    let router = Router::new();
    let router = add_handler::<VoteActive>(router);
    add_handler::<IsVoter>(router)
}
