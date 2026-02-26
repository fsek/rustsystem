/// Tests for the POST /server/api/start-round endpoint.
///
/// start-round is the only trustauth endpoint that does not call back to the server,
/// so all tests here run without a live server instance.
use axum::http::StatusCode;
use uuid::Uuid;

use crate::common::MockApp;
use crate::inprocess::{call_start_round, start_round_ok};

/// BBS+ over BLS12-381 uses a G2 element as the public key, which serialises to
/// exactly 96 bytes in compressed form.
const BLS12_381_PUB_KEY_LEN: usize = 96;

// ──────────────────────────────────────────────────────────────────────────────
// Basic happy-path tests
// ──────────────────────────────────────────────────────────────────────────────

/// POST /server/api/start-round with a valid body returns 200.
#[tokio::test]
async fn test_start_round_success() {
    let app = MockApp::new_inprocess();
    let muuid = Uuid::new_v4();
    let res = call_start_round(&app, muuid, "Budget vote").await;
    assert_eq!(res.status(), StatusCode::OK);
}

/// The response body contains a `pub_key_bytes` field that is non-empty.
#[tokio::test]
async fn test_start_round_returns_pub_key() {
    let app = MockApp::new_inprocess();
    let body = start_round_ok(&app, Uuid::new_v4(), "Budget vote").await;

    let bytes = body["pub_key_bytes"]
        .as_array()
        .expect("pub_key_bytes must be a JSON array");
    assert!(!bytes.is_empty(), "pub_key_bytes must not be empty");
}

/// The public key is exactly 96 bytes (compressed BLS12-381 G2 point).
#[tokio::test]
async fn test_start_round_pub_key_length() {
    let app = MockApp::new_inprocess();
    let body = start_round_ok(&app, Uuid::new_v4(), "Election").await;

    let bytes = body["pub_key_bytes"].as_array().unwrap();
    assert_eq!(
        bytes.len(),
        BLS12_381_PUB_KEY_LEN,
        "BBS+ public key must be {} bytes, got {}",
        BLS12_381_PUB_KEY_LEN,
        bytes.len()
    );
}

// ──────────────────────────────────────────────────────────────────────────────
// State persistence tests
// ──────────────────────────────────────────────────────────────────────────────

/// After a successful call the round is retrievable via AppState::get_round.
#[tokio::test]
async fn test_start_round_stores_round() {
    let app = MockApp::new_inprocess();
    let muuid = Uuid::new_v4();

    start_round_ok(&app, muuid, "Board vote").await;

    let round = app
        .state
        .get_round(muuid)
        .await
        .expect("round must be present after start-round");

    // The header is name.as_bytes().
    assert_eq!(round.header, b"Board vote", "header must match the name");
}

/// The header stored in RoundState equals the name supplied in the request,
/// encoded as UTF-8 bytes.
#[tokio::test]
async fn test_start_round_header_matches_name() {
    let app = MockApp::new_inprocess();
    let muuid = Uuid::new_v4();
    let name = "Säkerhetsfråga 2025";

    start_round_ok(&app, muuid, name).await;

    let round = app.state.get_round(muuid).await.unwrap();
    assert_eq!(round.header, name.as_bytes());
}

/// The public key returned in the response matches the one stored in the round.
#[tokio::test]
async fn test_start_round_pub_key_matches_stored() {
    let app = MockApp::new_inprocess();
    let muuid = Uuid::new_v4();
    let body = start_round_ok(&app, muuid, "Salary").await;

    let returned: Vec<u8> = body["pub_key_bytes"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_u64().unwrap() as u8)
        .collect();

    let round = app.state.get_round(muuid).await.unwrap();
    let stored = round.keys.public_key().to_bytes().to_vec();

    assert_eq!(returned, stored, "returned pub_key_bytes must match stored key");
}

// ──────────────────────────────────────────────────────────────────────────────
// Round-overwrite semantics
// ──────────────────────────────────────────────────────────────────────────────

/// Calling start-round twice for the same muuid replaces the first round.
/// The second response succeeds and stores a new (typically different) key.
#[tokio::test]
async fn test_start_round_overwrites_previous_round() {
    let app = MockApp::new_inprocess();
    let muuid = Uuid::new_v4();

    let body1 = start_round_ok(&app, muuid, "Round 1").await;
    let body2 = start_round_ok(&app, muuid, "Round 2").await;

    // The stored round's header should reflect the second call.
    let round = app.state.get_round(muuid).await.unwrap();
    assert_eq!(round.header, b"Round 2", "second call must overwrite the round");

    // Keys are randomly generated; they should (with overwhelming probability) differ.
    // We verify by comparing the returned bytes.
    let key1: Vec<u8> = body1["pub_key_bytes"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_u64().unwrap() as u8)
        .collect();
    let key2: Vec<u8> = body2["pub_key_bytes"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_u64().unwrap() as u8)
        .collect();
    assert_ne!(key1, key2, "each call should generate a fresh keypair");
}

// ──────────────────────────────────────────────────────────────────────────────
// Multi-meeting isolation tests
// ──────────────────────────────────────────────────────────────────────────────

/// Two rounds for different muuids coexist independently and return distinct keys.
#[tokio::test]
async fn test_start_round_independent_meetings() {
    let app = MockApp::new_inprocess();

    let muuid_a = Uuid::new_v4();
    let muuid_b = Uuid::new_v4();

    let body_a = start_round_ok(&app, muuid_a, "Meeting A").await;
    let body_b = start_round_ok(&app, muuid_b, "Meeting B").await;

    // Both rounds are stored separately.
    let round_a = app.state.get_round(muuid_a).await.unwrap();
    let round_b = app.state.get_round(muuid_b).await.unwrap();

    assert_eq!(round_a.header, b"Meeting A");
    assert_eq!(round_b.header, b"Meeting B");

    // Keys should differ between independent meetings.
    let key_a: Vec<u8> = body_a["pub_key_bytes"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_u64().unwrap() as u8)
        .collect();
    let key_b: Vec<u8> = body_b["pub_key_bytes"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_u64().unwrap() as u8)
        .collect();
    assert_ne!(key_a, key_b);
}

/// Starting rounds for 5 different meetings populates all 5 in the map.
#[tokio::test]
async fn test_start_round_multiple_meetings_stored() {
    let app = MockApp::new_inprocess();
    let muuids: Vec<Uuid> = (0..5).map(|_| Uuid::new_v4()).collect();

    for (i, &muuid) in muuids.iter().enumerate() {
        start_round_ok(&app, muuid, &format!("Meeting {i}")).await;
    }

    // Every meeting's round must be present.
    for &muuid in &muuids {
        app.state
            .get_round(muuid)
            .await
            .expect("round must be present for every muuid");
    }
}

// ──────────────────────────────────────────────────────────────────────────────
// Concurrent-access tests
// ──────────────────────────────────────────────────────────────────────────────

/// 10 concurrent start-round requests for 10 distinct meetings all succeed.
/// Every returned public key is unique (no key was reused across rounds).
#[tokio::test]
async fn test_start_round_concurrent_distinct_meetings() {
    let app = MockApp::new_inprocess();
    let muuids: Vec<Uuid> = (0..10).map(|_| Uuid::new_v4()).collect();

    let names: Vec<String> = (0..10).map(|i| format!("Concurrent {i}")).collect();
    let futures: Vec<_> = muuids
        .iter()
        .zip(names.iter())
        .map(|(&muuid, name)| start_round_ok(&app, muuid, name))
        .collect();

    let bodies: Vec<serde_json::Value> = futures::future::join_all(futures).await;

    // All 10 succeeded.
    assert_eq!(bodies.len(), 10);

    // Collect all public keys and check they are all distinct.
    let keys: Vec<Vec<u8>> = bodies
        .iter()
        .map(|body| {
            body["pub_key_bytes"]
                .as_array()
                .unwrap()
                .iter()
                .map(|v| v.as_u64().unwrap() as u8)
                .collect()
        })
        .collect();

    // Pairwise comparison: no two keys should be equal.
    for i in 0..keys.len() {
        for j in (i + 1)..keys.len() {
            assert_ne!(
                keys[i], keys[j],
                "keys for rounds {i} and {j} must be distinct"
            );
        }
    }

    // All 10 rounds must be stored.
    for &muuid in &muuids {
        app.state
            .get_round(muuid)
            .await
            .expect("round must be present after concurrent start-round");
    }
}

/// Concurrent start-round calls for the same muuid are safe: no panic, no
/// deadlock, and the final stored round has a valid 96-byte public key.
#[tokio::test]
async fn test_start_round_concurrent_same_meeting() {
    let app = MockApp::new_inprocess();
    let muuid = Uuid::new_v4();

    let names: Vec<String> = (0..5).map(|i| format!("Overwrite {i}")).collect();
    let futures: Vec<_> = names
        .iter()
        .map(|name| call_start_round(&app, muuid, name))
        .collect();

    let responses = futures::future::join_all(futures).await;

    // All requests must succeed.
    for res in &responses {
        assert_eq!(res.status(), StatusCode::OK);
    }

    // The final stored round must have a valid key.
    let round = app.state.get_round(muuid).await.unwrap();
    assert_eq!(
        round.keys.public_key().to_bytes().len(),
        BLS12_381_PUB_KEY_LEN
    );
}

// ──────────────────────────────────────────────────────────────────────────────
// Missing-field / bad-request tests
// ──────────────────────────────────────────────────────────────────────────────

/// A request body missing required fields returns 422 Unprocessable Entity.
#[tokio::test]
async fn test_start_round_missing_fields() {
    use axum::http::Method;
    use crate::common::json_request;

    let app = MockApp::new_inprocess();

    let res = app
        .oneshot(json_request(
            Method::POST,
            "/server/api/start-round",
            serde_json::json!({}), // no muuid, no name
        ))
        .await;

    assert_eq!(
        res.status(),
        StatusCode::UNPROCESSABLE_ENTITY,
        "missing fields should yield 422"
    );
}

/// A request body with only `muuid` (missing `name`) returns 422.
#[tokio::test]
async fn test_start_round_missing_name() {
    use axum::http::Method;
    use crate::common::json_request;

    let app = MockApp::new_inprocess();

    let res = app
        .oneshot(json_request(
            Method::POST,
            "/server/api/start-round",
            serde_json::json!({ "muuid": Uuid::new_v4() }),
        ))
        .await;

    assert_eq!(res.status(), StatusCode::UNPROCESSABLE_ENTITY);
}

/// A request body with an invalid UUID string returns 422.
#[tokio::test]
async fn test_start_round_invalid_uuid() {
    use axum::http::Method;
    use crate::common::json_request;

    let app = MockApp::new_inprocess();

    let res = app
        .oneshot(json_request(
            Method::POST,
            "/server/api/start-round",
            serde_json::json!({ "muuid": "not-a-valid-uuid", "name": "Test" }),
        ))
        .await;

    assert_eq!(res.status(), StatusCode::UNPROCESSABLE_ENTITY);
}
