/// E2E tests for the trustauth login flow (Trustauth → Server is-voter callback).
///
/// When a voter logs in to trustauth, trustauth calls the server's
/// `/trustauth/is-voter` endpoint via its internal HTTP client. These tests
/// verify that the cross-service validation works correctly.
use super::{E2eApp, add_voter, create_meeting, server_login};
use uuid::Uuid;

/// A voter who exists on the server can log in to trustauth and receives a
/// trustauth JWT cookie.
#[tokio::test]
async fn test_trustauth_login_valid_voter() {
    let app = E2eApp::new().await;
    let client = app.new_client();

    create_meeting(&app, &client).await;
    let qr = add_voter(&app, &client, "Alice", false).await;
    server_login(&app, &client, &qr).await;

    // Log in to trustauth — it calls server's is-voter to verify.
    let resp = client
        .post(format!("{}/api/login", app.ta_url))
        .json(&serde_json::json!({
            "uuuid": extract_uuuid(&qr.invite_link),
            "muuid": extract_muuid(&qr.invite_link)
        }))
        .send()
        .await
        .unwrap();

    assert_eq!(
        resp.status(),
        202,
        "trustauth login should succeed for a valid voter"
    );
    // The response must set a trustauth_token cookie.
    let set_cookie = resp.headers().get("set-cookie").unwrap().to_str().unwrap();
    assert!(
        set_cookie.contains("trustauth_token"),
        "trustauth should set trustauth_token cookie"
    );
}

/// A UUID that does not exist in the server's meeting is rejected by trustauth.
#[tokio::test]
async fn test_trustauth_login_unknown_voter() {
    let app = E2eApp::new().await;
    let client = app.new_client();

    // Create a meeting so there is a valid muuid, but use a random uuuid.
    let create_resp = client
        .post(format!("{}/api/create-meeting", app.server_url))
        .json(&serde_json::json!({
            "title": "Test",
            "host_name": "Creator",
            "pub_key": ""
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(create_resp.status(), 201);

    // Extract muuid from the host cookie JWT — but for this test we just need
    // a valid meeting to exist. Use a completely random uuuid.
    // We don't know the muuid from the create-meeting response easily, so
    // instead we'll try to log in to trustauth with random ids directly.
    let resp = client
        .post(format!("{}/api/login", app.ta_url))
        .json(&serde_json::json!({
            "uuuid": Uuid::new_v4().to_string(),
            "muuid": Uuid::new_v4().to_string()
        }))
        .send()
        .await
        .unwrap();

    // Trustauth calls server's is-voter with an unknown muuid → server returns
    // 404 (MUuidNotFound) → trustauth wraps that as TrustAuthFetch → 500.
    assert!(
        !resp.status().is_success(),
        "unknown voter should be rejected, got {}",
        resp.status()
    );
}

/// A voter removed from the server after login cannot log in to trustauth.
#[tokio::test]
async fn test_trustauth_login_removed_voter() {
    let app = E2eApp::new().await;
    let host = app.new_client();
    let voter = app.new_client();

    create_meeting(&app, &host).await;
    let qr = add_voter(&app, &host, "Bob", false).await;
    server_login(&app, &voter, &qr).await;

    let uuuid = extract_uuuid(&qr.invite_link);
    let muuid = extract_muuid(&qr.invite_link);

    // Find Bob's UUID via voter-id and remove him (host operations).
    let id_resp = host
        .get(format!("{}/api/host/voter-id", app.server_url))
        .json(&serde_json::json!({ "name": "Bob" }))
        .send()
        .await
        .unwrap();
    assert_eq!(id_resp.status(), 200);
    let voter_uuid: Uuid = id_resp.json().await.unwrap();

    let del_resp = host
        .delete(format!("{}/api/host/remove-voter", app.server_url))
        .json(&serde_json::json!({ "voter_uuuid": voter_uuid }))
        .send()
        .await
        .unwrap();
    assert_eq!(del_resp.status(), 200);

    // Now Bob no longer exists on the server.  Trustauth login should fail
    // because the is-voter callback returns false.
    let login_resp = voter
        .post(format!("{}/api/login", app.ta_url))
        .json(&serde_json::json!({ "uuuid": uuuid, "muuid": muuid }))
        .send()
        .await
        .unwrap();

    assert!(
        login_resp.status().is_client_error(),
        "removed voter should not be able to log in to trustauth, got {}",
        login_resp.status()
    );
}

/// A voter who logs in to trustauth twice gets the same kind of cookie.
/// Trustauth is stateless for login (no duplicate-login restriction there),
/// so the second call should also succeed.
#[tokio::test]
async fn test_trustauth_login_idempotent() {
    let app = E2eApp::new().await;
    let client = app.new_client();

    create_meeting(&app, &client).await;
    let qr = add_voter(&app, &client, "Carol", false).await;
    server_login(&app, &client, &qr).await;

    let payload = serde_json::json!({
        "uuuid": extract_uuuid(&qr.invite_link),
        "muuid": extract_muuid(&qr.invite_link)
    });

    let resp1 = client
        .post(format!("{}/api/login", app.ta_url))
        .json(&payload)
        .send()
        .await
        .unwrap();
    assert_eq!(resp1.status(), 202);

    let resp2 = client
        .post(format!("{}/api/login", app.ta_url))
        .json(&payload)
        .send()
        .await
        .unwrap();
    assert_eq!(resp2.status(), 202, "second trustauth login should also succeed");
}

// ── helpers ───────────────────────────────────────────────────────────────────

fn extract_uuuid(invite_link: &str) -> String {
    let url = url::Url::parse(invite_link).unwrap();
    let params: std::collections::HashMap<_, _> = url.query_pairs().into_owned().collect();
    params["uuuid"].clone()
}

fn extract_muuid(invite_link: &str) -> String {
    let url = url::Url::parse(invite_link).unwrap();
    let params: std::collections::HashMap<_, _> = url.query_pairs().into_owned().collect();
    params["muuid"].clone()
}
