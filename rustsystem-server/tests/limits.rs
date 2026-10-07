//! Request limits: body size and per-IP rate limits. These build the router with
//! `rate_limit_forced`, so `RUSTSYSTEM_DISABLE_RATE_LIMIT` in the environment can't turn
//! them into no-ops.

use std::net::{IpAddr, SocketAddr};

use axum::{
    Router,
    body::Body,
    extract::ConnectInfo,
    http::{Method, Request, StatusCode, header},
};
use http_body_util::BodyExt;
use serde_json::Value;
use tower::util::ServiceExt;

use rustsystem_core::limits::{CREATE_MEETING, ClientIp, GENERAL, rate_limit_forced};
use rustsystem_server::{AppState, RateLimits, Settings, router, trustauth::Trustauth};

const PROXY: [u8; 4] = [10, 0, 0, 1];

fn service() -> Router {
    let frontend = std::env::temp_dir().join(format!("rustsystem-limits-{}", uuid::Uuid::new_v4()));
    std::fs::create_dir_all(&frontend).unwrap();
    std::fs::write(frontend.join("index.html"), "<!doctype html>").unwrap();
    let settings = Settings {
        public_url: "http://localhost".into(),
        trustauth_public_url: "http://localhost:2443".into(),
        secure_cookies: false,
        meetings_dir: std::env::temp_dir(),
    };
    // Trustauth is never reached: every request here is refused before or after it matters.
    let app = AppState::new(settings, Trustauth::new(reqwest::Client::new(), "http://127.0.0.1:9"));
    let ips = ClientIp::new(vec![IpAddr::from(PROXY)]);
    let limits = RateLimits {
        general: Some(rate_limit_forced(GENERAL, ips.clone())),
        create_meeting: Some(rate_limit_forced(CREATE_MEETING, ips)),
    };
    router(app, &frontend, limits)
}

fn request(method: Method, uri: &str, body: Body, peer: [u8; 4], xff: Option<&str>) -> Request<Body> {
    let mut req = Request::builder()
        .method(method)
        .uri(uri)
        .header(header::CONTENT_TYPE, "application/json");
    if let Some(v) = xff {
        req = req.header("x-forwarded-for", v);
    }
    let mut req = req.body(body).unwrap();
    req.extensions_mut().insert(ConnectInfo(SocketAddr::from((peer, 40000))));
    req
}

/// An invalid create-meeting request: rejected after the rate limiter, never reaching trustauth.
fn create(peer: [u8; 4], xff: Option<&str>) -> Request<Body> {
    request(Method::POST, "/api/meetings", Body::from("{}"), peer, xff)
}

async fn status_and_code(router: &Router, req: Request<Body>) -> (StatusCode, String) {
    let res = router.clone().oneshot(req).await.unwrap();
    let status = res.status();
    let bytes = res.into_body().collect().await.unwrap().to_bytes();
    let json: Value = serde_json::from_slice(&bytes).unwrap_or(Value::Null);
    (status, json["code"].as_str().unwrap_or("").to_owned())
}

#[tokio::test]
async fn oversized_body_is_413() {
    let router = service();
    let req = request(Method::POST, "/api/login", Body::from(vec![b'x'; 128 * 1024]), [127, 0, 0, 1], None);
    assert_eq!(status_and_code(&router, req).await, (StatusCode::PAYLOAD_TOO_LARGE, "BodyTooLarge".into()));
}

#[tokio::test]
async fn creating_meetings_is_rate_limited_per_ip() {
    let router = service();
    let client = [203, 0, 113, 1];
    for _ in 0..CREATE_MEETING.burst {
        assert_ne!(status_and_code(&router, create(client, None)).await.0, StatusCode::TOO_MANY_REQUESTS);
    }
    assert_eq!(
        status_and_code(&router, create(client, None)).await,
        (StatusCode::TOO_MANY_REQUESTS, "RateLimited".into())
    );
    // Someone else is unaffected.
    assert_ne!(status_and_code(&router, create([203, 0, 113, 2], None)).await.0, StatusCode::TOO_MANY_REQUESTS);
}

#[tokio::test]
async fn forged_forwarded_for_does_not_buy_a_fresh_bucket() {
    let router = service();
    let client = [203, 0, 113, 3];
    for i in 0..=CREATE_MEETING.burst {
        let forged = format!("198.51.100.{i}");
        let (status, _) = status_and_code(&router, create(client, Some(&forged))).await;
        if i == CREATE_MEETING.burst {
            assert_eq!(status, StatusCode::TOO_MANY_REQUESTS, "X-Forwarded-For from a non-proxy is ignored");
        }
    }
}

#[tokio::test]
async fn behind_the_trusted_proxy_each_client_has_its_own_bucket() {
    let router = service();
    for _ in 0..CREATE_MEETING.burst {
        status_and_code(&router, create(PROXY, Some("198.51.100.20"))).await;
    }
    assert_eq!(status_and_code(&router, create(PROXY, Some("198.51.100.20"))).await.0, StatusCode::TOO_MANY_REQUESTS);
    assert_ne!(status_and_code(&router, create(PROXY, Some("198.51.100.21"))).await.0, StatusCode::TOO_MANY_REQUESTS);
}

#[tokio::test]
async fn static_assets_are_not_rate_limited() {
    let router = service();
    for _ in 0..GENERAL.burst + 50 {
        let req = request(Method::GET, "/meeting", Body::empty(), [203, 0, 113, 9], None);
        assert_eq!(router.clone().oneshot(req).await.unwrap().status(), StatusCode::OK);
    }
}
