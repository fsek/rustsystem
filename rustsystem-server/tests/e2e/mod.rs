/// End-to-end tests: both rustsystem-server and rustsystem-trustauth run
/// in the same tokio process, bound to random localhost ports. Inter-service
/// calls are plain HTTP (no mTLS) via the injected-URL constructors.
///
/// Test layout:
///   start_vote  — server calls trustauth's start-round when host starts a vote
///   auth        — trustauth's login calls server's is-voter callback
///   registration — register, is-registered, vote-data flows
///   lifecycle   — full meeting lifecycle smoke test (create → vote → tally)
mod auth;
mod lifecycle;
mod registration;
mod start_vote;

use rustsystem_server::{api::host::new_voter::QrCodeResponse, app_combined, new_test_state};
use rustsystem_trustauth::{app_combined as ta_app_combined, new_test_state as ta_new_test_state};
use serde::de::DeserializeOwned;
use tokio::net::TcpListener;
use url::Url;

// ── E2eApp ────────────────────────────────────────────────────────────────────

/// Both services running on random localhost ports. The services are spawned
/// as background tasks and aborted when this struct is dropped.
pub struct E2eApp {
    pub server_url: String,
    pub ta_url: String,
    _server_task: tokio::task::JoinHandle<()>,
    _ta_task: tokio::task::JoinHandle<()>,
}

impl E2eApp {
    pub async fn new() -> Self {
        let server_listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let ta_listener = TcpListener::bind("127.0.0.1:0").await.unwrap();

        let server_port = server_listener.local_addr().unwrap().port();
        let ta_port = ta_listener.local_addr().unwrap().port();

        let server_url = format!("http://127.0.0.1:{server_port}");
        let ta_url = format!("http://127.0.0.1:{ta_port}");

        let server_state = new_test_state(&ta_url);
        let ta_state = ta_new_test_state(&server_url);

        let server_router = app_combined(server_state);
        // app_combined returns Result; unwrap is acceptable in test scaffolding.
        let ta_router = ta_app_combined(ta_state).unwrap();

        let server_task = tokio::spawn(async move {
            axum::serve(server_listener, server_router).await.unwrap();
        });
        let ta_task = tokio::spawn(async move {
            axum::serve(ta_listener, ta_router).await.unwrap();
        });

        Self {
            server_url,
            ta_url,
            _server_task: server_task,
            _ta_task: ta_task,
        }
    }

    /// Creates a fresh `reqwest::Client` with a cookie jar. All cookies received
    /// from the server (access_token) and trustauth (trustauth_token) are
    /// automatically stored and resent to the appropriate service on subsequent
    /// requests. Having both cookies present on requests is harmless — each
    /// service ignores the cookie it doesn't own.
    pub fn new_client(&self) -> reqwest::Client {
        reqwest::Client::builder()
            .cookie_store(true)
            .build()
            .unwrap()
    }
}

impl Drop for E2eApp {
    fn drop(&mut self) {
        self._server_task.abort();
        self._ta_task.abort();
    }
}

// ── HTTP helpers ──────────────────────────────────────────────────────────────

/// Parse a JSON response body into `T`, panicking with the raw bytes on failure.
pub async fn parse_body<T: DeserializeOwned>(resp: reqwest::Response) -> T {
    let bytes = resp.bytes().await.unwrap();
    serde_json::from_slice(&bytes)
        .unwrap_or_else(|e| panic!("failed to parse response body: {e}\nraw: {bytes:?}"))
}

// ── High-level scenario helpers ───────────────────────────────────────────────

/// POST /api/create-meeting and return the host JWT cookie header value.
pub async fn create_meeting(app: &E2eApp, client: &reqwest::Client) -> String {
    let resp = client
        .post(format!("{}/api/create-meeting", app.server_url))
        .json(&serde_json::json!({
            "title": "Test Meeting",
            "host_name": "Creator",
            "pub_key": ""
        }))
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), 201, "create-meeting should return 201");
    // Return the Set-Cookie value so callers that need it explicitly can use it.
    resp.headers()
        .get("set-cookie")
        .unwrap()
        .to_str()
        .unwrap()
        .to_owned()
}

/// POST /api/host/new-voter and return the parsed QrCodeResponse.
pub async fn add_voter(
    app: &E2eApp,
    client: &reqwest::Client,
    name: &str,
    is_host: bool,
) -> QrCodeResponse {
    let resp = client
        .post(format!("{}/api/host/new-voter", app.server_url))
        .json(&serde_json::json!({ "voterName": name, "isHost": is_host }))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 201, "new-voter should return 201");
    parse_body(resp).await
}

/// POST /api/login to the **server**, logging in a voter by their invite link.
/// Returns the Set-Cookie header string.
pub async fn server_login(app: &E2eApp, client: &reqwest::Client, qr: &QrCodeResponse) -> String {
    let url = Url::parse(&qr.invite_link).unwrap();
    let params: std::collections::HashMap<_, _> = url.query_pairs().into_owned().collect();

    let resp = client
        .post(format!("{}/api/login", app.server_url))
        .json(&serde_json::json!({
            "uuuid": params["uuuid"],
            "muuid": params["muuid"],
            "admin_cred": null
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 202, "server login should return 202");
    resp.headers()
        .get("set-cookie")
        .map(|v| v.to_str().unwrap().to_owned())
        .unwrap_or_default()
}

/// POST /api/login to **trustauth**, logging a voter in using the same invite link.
pub async fn ta_login(app: &E2eApp, client: &reqwest::Client, qr: &QrCodeResponse) {
    let url = Url::parse(&qr.invite_link).unwrap();
    let params: std::collections::HashMap<_, _> = url.query_pairs().into_owned().collect();

    let resp = client
        .post(format!("{}/api/login", app.ta_url))
        .json(&serde_json::json!({
            "uuuid": params["uuuid"],
            "muuid": params["muuid"]
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 202, "trustauth login should return 202");
}

/// POST /api/host/start-vote.
pub async fn start_vote(
    app: &E2eApp,
    client: &reqwest::Client,
    vote_name: &str,
    candidates: Vec<String>,
) {
    let resp = client
        .post(format!("{}/api/host/start-vote", app.server_url))
        .json(&serde_json::json!({
            "name": vote_name,
            "shuffle": false,
            "metadata": {
                "candidates": candidates,
                "max_choices": 1,
                "protocol_version": 1
            }
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200, "start-vote should return 200");
}

/// GET /api/common/vote-progress.
pub async fn vote_progress(app: &E2eApp, client: &reqwest::Client) -> serde_json::Value {
    let resp = client
        .get(format!("{}/api/common/vote-progress", app.server_url))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);
    parse_body(resp).await
}
