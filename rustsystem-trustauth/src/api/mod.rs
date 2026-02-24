use rustsystem_core::add_handler;
use axum::Router;

use crate::AppState;

mod login;
use login::Login;

mod register;
use register::Register;

mod is_registered;
use is_registered::IsRegistered;

mod vote_data;
use vote_data::GetVoteData;

pub fn public_routes() -> Router<AppState> {
    let mut router = Router::new();
    router = add_handler::<Login>(router);
    router = add_handler::<Register>(router);
    router = add_handler::<IsRegistered>(router);
    router = add_handler::<GetVoteData>(router);
    router
}
