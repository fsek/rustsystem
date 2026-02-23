use api_core::add_handler;
use axum::Router;

use crate::{AppState, api::register::IsRegistered};

mod login;
use login::Login;

mod register;
use register::Register;

mod start_round;
use start_round::StartRound;

pub fn trustauth_routes() -> Router<AppState> {
    let mut router = Router::new();
    router = add_handler::<Login>(router);
    router = add_handler::<Register>(router);
    router = add_handler::<IsRegistered>(router);
    router = add_handler::<StartRound>(router);
    router
}
