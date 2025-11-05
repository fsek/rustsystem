use askama::Template;
use askama_web::WebTemplate;
use axum::{Router, response::IntoResponse, routing::get};
use uuid::Uuid;

use crate::AppState;

mod admin;
mod voter;

#[derive(Debug, serde::Deserialize)]
struct MeetingPath {
    meeting_id: Uuid,
}

#[derive(Template, WebTemplate)]
#[template(path = "home.html")]
struct HomePage {}

async fn index() -> impl IntoResponse {
    HomePage {}
}

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/", get(index))
        .merge(admin::router())
        .merge(voter::router())
}
