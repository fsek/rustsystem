use axum::http::Method;
use serde_json::json;
use uuid::Uuid;

use crate::common::{MockApp, json_request, parse_response_body};

mod start_round;

pub async fn call_start_round(
    app: &MockApp,
    muuid: Uuid,
    name: &str,
) -> axum::response::Response {
    app.oneshot(json_request(
        Method::POST,
        "/server/api/start-round",
        json!({ "muuid": muuid, "name": name }),
    ))
    .await
}

/// Calls start-round, asserts 200, returns the parsed JSON body.
pub async fn start_round_ok(app: &MockApp, muuid: Uuid, name: &str) -> serde_json::Value {
    let res = call_start_round(app, muuid, name).await;
    assert_eq!(
        res.status(),
        axum::http::StatusCode::OK,
        "start-round should return 200"
    );
    parse_response_body(res).await
}
