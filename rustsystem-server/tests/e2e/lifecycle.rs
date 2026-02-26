/// Full meeting lifecycle smoke tests.
///
/// These tests drive the complete flow from meeting creation through vote
/// tallying, exercising every cross-service call in sequence.
use super::{E2eApp, add_voter, create_meeting, server_login, start_vote, ta_login, vote_progress};

// ── helpers ───────────────────────────────────────────────────────────────────

/// Create a commitment payload and register with trustauth. Returns the
/// blind signature data from vote-data.
async fn register_and_get_vote_data(
    app: &E2eApp,
    client: &reqwest::Client,
) -> serde_json::Value {
    use zkryptium::schemes::{algorithms::BbsBls12381Sha256, generics::Commitment};

    let (commitment, blind_factor) =
        Commitment::<BbsBls12381Sha256>::commit(None).unwrap();

    let payload = serde_json::json!({
        "commitment": commitment,
        "token": vec![0u8; 32],
        "blind_factor": blind_factor.to_bytes().to_vec(),
        "context": {}
    });

    let reg_resp = client
        .post(format!("{}/api/register", app.ta_url))
        .json(&payload)
        .send()
        .await
        .unwrap();
    assert_eq!(reg_resp.status(), 201, "registration should succeed");

    let data_resp = client
        .get(format!("{}/api/vote-data", app.ta_url))
        .send()
        .await
        .unwrap();
    assert_eq!(data_resp.status(), 200);
    data_resp.json().await.unwrap()
}

// ── smoke tests ───────────────────────────────────────────────────────────────

/// Full lifecycle: create → add voter → login → start-vote → register →
/// vote-data → end-vote-round.
///
/// This test does not cast an actual cryptographic vote (that requires the
/// unblinding step which is covered in the crypto tests). It verifies that
/// the cross-service state transitions work end-to-end.
#[tokio::test]
async fn test_full_lifecycle_create_to_end_vote() {
    let app = E2eApp::new().await;
    let host = app.new_client();
    let voter = app.new_client();

    // 1. Create meeting (host gets access_token cookie).
    create_meeting(&app, &host).await;

    // 2. Add voter and log them in to both services.
    let qr = add_voter(&app, &host, "Alice", false).await;
    server_login(&app, &voter, &qr).await;
    ta_login(&app, &voter, &qr).await;

    // 3. Host starts vote → server calls trustauth's start-round.
    start_vote(&app, &host, "Budget 2025", vec!["Approve".into(), "Reject".into()]).await;

    let progress = vote_progress(&app, &host).await;
    assert_eq!(progress["isActive"], true);
    assert_eq!(progress["totalParticipants"], 2); // host + Alice

    // 4. Voter registers → trustauth calls server's vote-active + is-voter.
    let _vote_data = register_and_get_vote_data(&app, &voter).await;

    // 5. Verify registration is reflected.
    let is_reg_resp = voter
        .get(format!("{}/api/is-registered", app.ta_url))
        .send()
        .await
        .unwrap();
    assert_eq!(is_reg_resp.status(), 200);
    let is_reg: serde_json::Value = is_reg_resp.json().await.unwrap();
    assert_eq!(is_reg["isRegistered"], true);

    // 6. Host ends vote round.
    let end_resp = host
        .delete(format!("{}/api/host/end-vote-round", app.server_url))
        .send()
        .await
        .unwrap();
    assert_eq!(end_resp.status(), 200);

    let progress_after = vote_progress(&app, &host).await;
    assert_eq!(progress_after["isActive"], false);
}

/// After a round ends, start-vote can be called again to begin a new round.
/// The new round on trustauth replaces the previous one with a fresh keypair.
#[tokio::test]
async fn test_lifecycle_multiple_rounds() {
    let app = E2eApp::new().await;
    let host = app.new_client();

    create_meeting(&app, &host).await;

    // Round 1.
    start_vote(&app, &host, "Round 1", vec!["Yes".into(), "No".into()]).await;
    assert_eq!(vote_progress(&app, &host).await["voteName"], "Round 1");

    let end1 = host
        .delete(format!("{}/api/host/end-vote-round", app.server_url))
        .send()
        .await
        .unwrap();
    assert_eq!(end1.status(), 200);

    // Round 2.
    start_vote(&app, &host, "Round 2", vec!["Option A".into(), "Option B".into()]).await;
    assert_eq!(vote_progress(&app, &host).await["voteName"], "Round 2");

    let end2 = host
        .delete(format!("{}/api/host/end-vote-round", app.server_url))
        .send()
        .await
        .unwrap();
    assert_eq!(end2.status(), 200);

    assert_eq!(vote_progress(&app, &host).await["isActive"], false);
}

/// After close-meeting, all endpoints are inaccessible with the old cookie.
#[tokio::test]
async fn test_lifecycle_close_meeting_revokes_access() {
    let app = E2eApp::new().await;
    let host = app.new_client();

    create_meeting(&app, &host).await;

    let close_resp = host
        .delete(format!("{}/api/host/close-meeting", app.server_url))
        .send()
        .await
        .unwrap();
    assert_eq!(close_resp.status(), 200);

    // After close, the cookie is implicitly revoked (meeting no longer exists).
    let voter_list_resp = host
        .get(format!("{}/api/host/voter-list", app.server_url))
        .send()
        .await
        .unwrap();
    assert_eq!(voter_list_resp.status(), 401);
}

/// Multiple voters can register concurrently without corrupting each other's data.
#[tokio::test]
async fn test_lifecycle_concurrent_registrations() {
    let app = E2eApp::new().await;
    let host = app.new_client();

    create_meeting(&app, &host).await;

    const N: usize = 5;

    // Add N voters, log each in.
    let mut voter_clients = Vec::new();
    for i in 0..N {
        let client = app.new_client();
        let qr = add_voter(&app, &host, &format!("Voter{i}"), false).await;
        server_login(&app, &client, &qr).await;
        ta_login(&app, &client, &qr).await;
        voter_clients.push(client);
    }

    start_vote(&app, &host, "Concurrent Vote", vec!["Yes".into(), "No".into()]).await;

    // All voters register concurrently.
    use zkryptium::schemes::{algorithms::BbsBls12381Sha256, generics::Commitment};

    let futures: Vec<_> = voter_clients.iter().map(|client| {
        let ta_url = app.ta_url.clone();
        async move {
            let (commitment, blind_factor) =
                Commitment::<BbsBls12381Sha256>::commit(None).unwrap();

            let payload = serde_json::json!({
                "commitment": commitment,
                "token": vec![0u8; 32],
                "blind_factor": blind_factor.to_bytes().to_vec(),
                "context": {}
            });

            client
                .post(format!("{ta_url}/api/register"))
                .json(&payload)
                .send()
                .await
                .unwrap()
        }
    }).collect();

    let responses = futures::future::join_all(futures).await;
    for (i, resp) in responses.iter().enumerate() {
        assert_eq!(
            resp.status(),
            201,
            "concurrent registration for voter {i} should succeed"
        );
    }

    // All N voters are now registered.
    for client in &voter_clients {
        let check = client
            .get(format!("{}/api/is-registered", app.ta_url))
            .send()
            .await
            .unwrap();
        assert_eq!(check.status(), 200);
        let body: serde_json::Value = check.json().await.unwrap();
        assert_eq!(body["isRegistered"], true);
    }
}
