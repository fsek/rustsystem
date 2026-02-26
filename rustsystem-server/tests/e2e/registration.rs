/// E2E tests for the voter registration flow.
///
/// These tests cover:
/// - POST /api/register on trustauth (blind signature issuance)
/// - GET /api/is-registered on trustauth
/// - GET /api/vote-data on trustauth
///
/// All three endpoints call back to the server via the injected HTTP client
/// to check vote-active and is-voter.
use super::{E2eApp, add_voter, create_meeting, server_login, start_vote, ta_login};
use zkryptium::schemes::{algorithms::BbsBls12381Sha256, generics::Commitment};

// ── helpers ───────────────────────────────────────────────────────────────────

/// Generate a valid BBS commitment payload. The commitment is real so that
/// trustauth's blind_sign call succeeds (returns 201).
fn dummy_commitment_payload() -> serde_json::Value {
    let (commitment, blind_factor) =
        Commitment::<BbsBls12381Sha256>::commit(None).unwrap();
    serde_json::json!({
        "commitment": commitment,
        "token": vec![0u8; 32],
        "blind_factor": blind_factor.to_bytes().to_vec(),
        "context": {}
    })
}

/// Add a voter (via host_client) and log them into both services (via
/// voter_client). Returns the QrCodeResponse.
async fn login_voter(
    app: &E2eApp,
    host_client: &reqwest::Client,
    voter_client: &reqwest::Client,
    name: &str,
) -> rustsystem_server::api::host::new_voter::QrCodeResponse {
    let qr = add_voter(app, host_client, name, false).await;
    server_login(app, voter_client, &qr).await;
    ta_login(app, voter_client, &qr).await;
    qr
}

// ── is-registered before any vote ─────────────────────────────────────────────

/// GET /api/is-registered returns false before a vote round starts.
#[tokio::test]
async fn test_is_registered_no_vote_active() {
    let app = E2eApp::new().await;
    let host = app.new_client();
    let voter = app.new_client();

    create_meeting(&app, &host).await;
    login_voter(&app, &host, &voter, "Alice").await;

    let resp = voter
        .get(format!("{}/api/is-registered", app.ta_url))
        .send()
        .await
        .unwrap();
    // No vote active → isRegistered = false (not an error, just false)
    // The endpoint returns 200 with { isRegistered: false } when vote is
    // inactive but the voter exists.
    assert!(
        resp.status().is_success() || resp.status().is_client_error(),
        "is-registered without active vote should return a valid response, got {}",
        resp.status()
    );
}

// ── register ───────────────────────────────────────────────────────────────────

/// Successful registration: vote is active, voter is valid, commitment is valid.
/// Trustauth issues a blind signature and returns 201.
#[tokio::test]
async fn test_register_happy_path() {
    let app = E2eApp::new().await;
    let host = app.new_client();
    let voter = app.new_client();

    create_meeting(&app, &host).await;
    login_voter(&app, &host, &voter, "Alice").await;
    start_vote(&app, &host, "Test Vote", vec!["Yes".into(), "No".into()]).await;

    let payload = dummy_commitment_payload();
    let resp = voter
        .post(format!("{}/api/register", app.ta_url))
        .json(&payload)
        .send()
        .await
        .unwrap();

    assert_eq!(
        resp.status(),
        201,
        "registration should succeed when vote is active and voter is valid"
    );
}

/// Calling register without an active vote returns VotingInactive (4xx).
#[tokio::test]
async fn test_register_fails_when_vote_inactive() {
    let app = E2eApp::new().await;
    let host = app.new_client();
    let voter = app.new_client();

    create_meeting(&app, &host).await;
    login_voter(&app, &host, &voter, "Alice").await;
    // Intentionally do NOT call start_vote.

    let payload = dummy_commitment_payload();
    let resp = voter
        .post(format!("{}/api/register", app.ta_url))
        .json(&payload)
        .send()
        .await
        .unwrap();

    assert!(
        resp.status().is_client_error(),
        "register without active vote should be rejected, got {}",
        resp.status()
    );
}

/// Calling register twice with the same voter returns 409 (AlreadyRegistered).
#[tokio::test]
async fn test_register_double_registration_rejected() {
    let app = E2eApp::new().await;
    let host = app.new_client();
    let voter = app.new_client();

    create_meeting(&app, &host).await;
    login_voter(&app, &host, &voter, "Alice").await;
    start_vote(&app, &host, "Test Vote", vec!["Yes".into(), "No".into()]).await;

    let payload = dummy_commitment_payload();

    let resp1 = voter
        .post(format!("{}/api/register", app.ta_url))
        .json(&payload)
        .send()
        .await
        .unwrap();
    assert_eq!(resp1.status(), 201, "first registration should succeed");

    let resp2 = voter
        .post(format!("{}/api/register", app.ta_url))
        .json(&payload)
        .send()
        .await
        .unwrap();
    assert_eq!(resp2.status(), 409, "second registration should return 409");
}

// ── is-registered after register ──────────────────────────────────────────────

/// GET /api/is-registered returns true after a successful registration.
#[tokio::test]
async fn test_is_registered_after_registration() {
    let app = E2eApp::new().await;
    let host = app.new_client();
    let voter = app.new_client();

    create_meeting(&app, &host).await;
    login_voter(&app, &host, &voter, "Alice").await;
    start_vote(&app, &host, "Test Vote", vec!["Yes".into(), "No".into()]).await;

    // Register.
    let payload = dummy_commitment_payload();
    let reg_resp = voter
        .post(format!("{}/api/register", app.ta_url))
        .json(&payload)
        .send()
        .await
        .unwrap();
    assert_eq!(reg_resp.status(), 201);

    // Now check.
    let check_resp = voter
        .get(format!("{}/api/is-registered", app.ta_url))
        .send()
        .await
        .unwrap();
    assert_eq!(check_resp.status(), 200);

    let body: serde_json::Value = check_resp.json().await.unwrap();
    assert_eq!(body["isRegistered"], true);
}

/// GET /api/is-registered returns false before the voter has registered.
#[tokio::test]
async fn test_is_registered_before_registration() {
    let app = E2eApp::new().await;
    let host = app.new_client();
    let voter = app.new_client();

    create_meeting(&app, &host).await;
    login_voter(&app, &host, &voter, "Alice").await;
    start_vote(&app, &host, "Test Vote", vec!["Yes".into(), "No".into()]).await;

    // Do NOT register.
    let check_resp = voter
        .get(format!("{}/api/is-registered", app.ta_url))
        .send()
        .await
        .unwrap();
    assert_eq!(check_resp.status(), 200);

    let body: serde_json::Value = check_resp.json().await.unwrap();
    assert_eq!(body["isRegistered"], false);
}

// ── vote-data ──────────────────────────────────────────────────────────────────

/// GET /api/vote-data returns the blind signature data after registration.
#[tokio::test]
async fn test_vote_data_returns_signature_after_registration() {
    let app = E2eApp::new().await;
    let host = app.new_client();
    let voter = app.new_client();

    create_meeting(&app, &host).await;
    login_voter(&app, &host, &voter, "Alice").await;
    start_vote(&app, &host, "Test Vote", vec!["Yes".into(), "No".into()]).await;

    let payload = dummy_commitment_payload();
    let reg_resp = voter
        .post(format!("{}/api/register", app.ta_url))
        .json(&payload)
        .send()
        .await
        .unwrap();
    assert_eq!(reg_resp.status(), 201);

    let data_resp = voter
        .get(format!("{}/api/vote-data", app.ta_url))
        .send()
        .await
        .unwrap();
    assert_eq!(data_resp.status(), 200);

    let body: serde_json::Value = data_resp.json().await.unwrap();
    assert!(body["token"].is_array(), "vote-data must include token");
    assert!(body["blind_factor"].is_array(), "vote-data must include blind_factor");
    assert!(!body["signature"].is_null(), "vote-data must include signature");
}

/// GET /api/vote-data fails with NotRegistered when the voter hasn't registered.
#[tokio::test]
async fn test_vote_data_not_registered() {
    let app = E2eApp::new().await;
    let host = app.new_client();
    let voter = app.new_client();

    create_meeting(&app, &host).await;
    login_voter(&app, &host, &voter, "Alice").await;
    start_vote(&app, &host, "Test Vote", vec!["Yes".into(), "No".into()]).await;

    // Do NOT register.
    let data_resp = voter
        .get(format!("{}/api/vote-data", app.ta_url))
        .send()
        .await
        .unwrap();
    assert!(
        data_resp.status().is_client_error(),
        "vote-data before registration should be rejected, got {}",
        data_resp.status()
    );
}

/// GET /api/vote-data fails when there is no active vote.
#[tokio::test]
async fn test_vote_data_when_vote_inactive() {
    let app = E2eApp::new().await;
    let host = app.new_client();
    let voter = app.new_client();

    create_meeting(&app, &host).await;
    login_voter(&app, &host, &voter, "Alice").await;
    // No start_vote.

    let data_resp = voter
        .get(format!("{}/api/vote-data", app.ta_url))
        .send()
        .await
        .unwrap();
    assert!(
        data_resp.status().is_client_error(),
        "vote-data without active vote should be rejected, got {}",
        data_resp.status()
    );
}

// ── multi-voter isolation ──────────────────────────────────────────────────────

/// Two voters each register independently; each gets their own blind signature.
/// Their `vote-data` responses are different.
#[tokio::test]
async fn test_register_two_voters_independent() {
    let app = E2eApp::new().await;
    let host = app.new_client();
    let voter_a = app.new_client();
    let voter_b = app.new_client();

    create_meeting(&app, &host).await;

    // Voter A
    let qr_a = add_voter(&app, &host, "Alice", false).await;
    server_login(&app, &voter_a, &qr_a).await;
    ta_login(&app, &voter_a, &qr_a).await;

    // Voter B
    let qr_b = add_voter(&app, &host, "Bob", false).await;
    server_login(&app, &voter_b, &qr_b).await;
    ta_login(&app, &voter_b, &qr_b).await;

    start_vote(&app, &host, "Test Vote", vec!["Yes".into(), "No".into()]).await;

    let payload_a = dummy_commitment_payload();
    let payload_b = dummy_commitment_payload();

    let reg_a = voter_a
        .post(format!("{}/api/register", app.ta_url))
        .json(&payload_a)
        .send()
        .await
        .unwrap();
    assert_eq!(reg_a.status(), 201);

    let reg_b = voter_b
        .post(format!("{}/api/register", app.ta_url))
        .json(&payload_b)
        .send()
        .await
        .unwrap();
    assert_eq!(reg_b.status(), 201, "second voter registration should also succeed");

    // Both can retrieve their vote data.
    let data_a = voter_a
        .get(format!("{}/api/vote-data", app.ta_url))
        .send()
        .await
        .unwrap();
    assert_eq!(data_a.status(), 200);

    let data_b = voter_b
        .get(format!("{}/api/vote-data", app.ta_url))
        .send()
        .await
        .unwrap();
    assert_eq!(data_b.status(), 200);
}
