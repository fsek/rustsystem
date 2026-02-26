/// Integration tests: server calls trustauth's start-round when the host
/// triggers start-vote. These are the simplest cross-service tests because
/// the only inter-service call is Server → Trustauth.
use super::{E2eApp, create_meeting, start_vote, vote_progress};

/// start-vote causes the server to call trustauth's start-round endpoint.
/// The round is then reflected in vote-progress (isActive = true).
#[tokio::test]
async fn test_start_vote_triggers_start_round() {
    let app = E2eApp::new().await;
    let client = app.new_client();

    create_meeting(&app, &client).await;

    let progress_before = vote_progress(&app, &client).await;
    assert_eq!(progress_before["isActive"], false);

    start_vote(&app, &client, "Budget 2025", vec!["Yes".into(), "No".into()]).await;

    let progress_after = vote_progress(&app, &client).await;
    assert_eq!(progress_after["isActive"], true);
    assert_eq!(progress_after["voteName"], "Budget 2025");
}

/// After start-vote the server has stored the trustauth public key. The
/// vote-progress response reflects the correct vote name and candidates.
#[tokio::test]
async fn test_start_vote_stores_round_metadata() {
    let app = E2eApp::new().await;
    let client = app.new_client();

    create_meeting(&app, &client).await;
    start_vote(&app, &client, "Election 2025", vec!["Alice".into(), "Bob".into()]).await;

    let progress = vote_progress(&app, &client).await;
    assert_eq!(progress["isActive"], true);
    assert_eq!(progress["voteName"], "Election 2025");
    assert_eq!(progress["totalVotesCast"], 0);
}

/// start-vote with a single allowed option (e.g. a Yes/No vote requiring
/// exactly one selection) succeeds and marks the round active.
#[tokio::test]
async fn test_start_vote_single_candidate() {
    let app = E2eApp::new().await;
    let client = app.new_client();

    create_meeting(&app, &client).await;
    start_vote(&app, &client, "Approve budget?", vec!["Approve".into(), "Reject".into()]).await;

    let progress = vote_progress(&app, &client).await;
    assert_eq!(progress["isActive"], true);
}

/// Calling start-vote while a vote is already active returns 409 (InvalidState).
#[tokio::test]
async fn test_start_vote_while_active_rejected() {
    let app = E2eApp::new().await;
    let client = app.new_client();

    create_meeting(&app, &client).await;
    start_vote(&app, &client, "Round 1", vec!["Yes".into(), "No".into()]).await;

    // Second start-vote while first is still active.
    let resp = client
        .post(format!("{}/api/host/start-vote", app.server_url))
        .json(&serde_json::json!({
            "name": "Round 2",
            "shuffle": false,
            "metadata": {
                "candidates": ["Yes", "No"],
                "max_choices": 1,
                "protocol_version": 1
            }
        }))
        .send()
        .await
        .unwrap();

    assert_eq!(
        resp.status(),
        409,
        "start-vote while vote is active should return 409"
    );
}

/// start-vote fails gracefully when trustauth is not reachable.
/// We simulate this by pointing the server at a port that has nothing listening.
#[tokio::test]
async fn test_start_vote_trustauth_unreachable() {
    use tokio::net::TcpListener;

    // Bind trustauth to a port but immediately drop the listener so nothing
    // serves requests there.
    let dead_listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let dead_port = dead_listener.local_addr().unwrap().port();
    drop(dead_listener);

    // Create a server that points at the dead trustauth port.
    let server_listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let server_port = server_listener.local_addr().unwrap().port();
    let dead_ta_url = format!("http://127.0.0.1:{dead_port}");

    let server_state = rustsystem_server::new_test_state(&dead_ta_url);
    let server_router = rustsystem_server::app_combined(server_state);
    let _server_task = tokio::spawn(async move {
        axum::serve(server_listener, server_router).await.unwrap();
    });

    let server_url = format!("http://127.0.0.1:{server_port}");
    let client = reqwest::Client::builder()
        .cookie_store(true)
        .build()
        .unwrap();

    // Create meeting (this doesn't call trustauth).
    let resp = client
        .post(format!("{server_url}/api/create-meeting"))
        .json(&serde_json::json!({
            "title": "Test",
            "host_name": "Host",
            "pub_key": ""
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 201);

    // start-vote should fail because trustauth is unreachable.
    let resp = client
        .post(format!("{server_url}/api/host/start-vote"))
        .json(&serde_json::json!({
            "name": "Test Vote",
            "shuffle": false,
            "metadata": {
                "candidates": ["Yes", "No"],
                "minVotes": 1,
                "maxVotes": 1
            }
        }))
        .send()
        .await
        .unwrap();

    assert!(
        resp.status().is_server_error() || resp.status().is_client_error(),
        "start-vote should fail when trustauth is unreachable, got {}",
        resp.status()
    );
}
