use api_core::add_handler;
use axum::Router;

use crate::{AppState, api::register::IsRegistered};

mod login;
use login::Login;

mod register;
use register::Register;

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
