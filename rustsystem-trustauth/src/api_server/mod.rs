mod start_round;
use rustsystem_core::add_handler;
use axum::Router;
use start_round::StartRound;

use crate::AppState;

pub fn internal_routes() -> Router<AppState> {
    let mut router = Router::new();
    router = add_handler::<StartRound>(router);
    router
}
