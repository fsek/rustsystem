use axum::{
    Router,
    routing::{MethodRouter, Route, get, post},
};

use crate::AppState;

mod create_meeting;
use create_meeting::create_meeting;

mod auth;
use auth::auth_meeting;

pub fn api_routes() -> Router<AppState> {
    Router::new()
        .route("/create-meeting", post(create_meeting))
        .route("/auth-meeting", post(auth_meeting))
}
