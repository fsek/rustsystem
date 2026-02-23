use axum::Router;
use status::IsSubmitted;
use vote::Submit;

use api_core::add_handler;

use crate::AppState;

mod status;
mod vote;

// Routes at /api/voter/...
pub fn voter_routes() -> Router<AppState> {
    let mut router = Router::new();
    router = add_handler::<Submit>(router);
    router = add_handler::<IsSubmitted>(router);
    router
}
