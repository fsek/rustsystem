/// Concurrency tests: verify that concurrent requests against the same shared AppState
/// don't corrupt data or deadlock.
///
/// These tests use `futures::future::join_all` to drive multiple requests concurrently
/// within a single async task. Because all server state is behind `AsyncRwLock`s, the
/// futures interleave at every `.await` point, exercising lock contention paths without
/// needing OS threads.
use axum::http::{Method, StatusCode};

use crate::{
    common::{MockApp, json_request},
    inprocess::{create_meeting, extract_cookie, parse_response_body, voter_list},
};
use rustsystem_server::api::host::{
    new_voter::NewVoterRequestBody, voter_list::VoterInfo,
};

/// 20 concurrent add-voter requests must all succeed and the voter list must reflect
/// exactly those 20 additions plus the host.
#[tokio::test]
async fn test_concurrent_add_voter() {
    let app = MockApp::new_inprocess();

    let creation_res = create_meeting(&app).await;
    let cookie = extract_cookie(&creation_res).1;

    let futures: Vec<_> = (0..20)
        .map(|i| {
            app.oneshot(json_request(
                Method::POST,
                "/api/host/new-voter",
                serde_json::to_value(NewVoterRequestBody {
                    voter_name: format!("ConcurrentVoter{i}"),
                    is_host: false,
                })
                .unwrap(),
                Some(cookie.clone()),
            ))
        })
        .collect();

    let responses = futures::future::join_all(futures).await;
    for res in &responses {
        assert_eq!(
            res.status(),
            StatusCode::CREATED,
            "every concurrent add-voter should succeed"
        );
    }

    let list_res = voter_list(&app, cookie).await;
    let voters = parse_response_body::<Vec<VoterInfo>>(list_res).await;
    assert_eq!(voters.len(), 21, "20 voters + 1 host");
}

/// Concurrent voter-list reads while voters are being added must not panic or return
/// corrupt data (e.g. partial reads).
#[tokio::test]
async fn test_concurrent_reads_during_writes() {
    let app = MockApp::new_inprocess();

    let creation_res = create_meeting(&app).await;
    let cookie = extract_cookie(&creation_res).1;

    // Interleave 10 writes (add-voter) with 10 reads (voter-list).
    let write_futures: Vec<_> = (0..10)
        .map(|i| {
            app.oneshot(json_request(
                Method::POST,
                "/api/host/new-voter",
                serde_json::to_value(NewVoterRequestBody {
                    voter_name: format!("RaceVoter{i}"),
                    is_host: false,
                })
                .unwrap(),
                Some(cookie.clone()),
            ))
        })
        .collect();

    let read_futures: Vec<_> = (0..10)
        .map(|_| {
            app.oneshot(json_request(
                Method::GET,
                "/api/host/voter-list",
                serde_json::to_value(()).unwrap(),
                Some(cookie.clone()),
            ))
        })
        .collect();

    // Drive all reads and writes concurrently.
    let all_futures: Vec<_> = write_futures.into_iter().chain(read_futures).collect();
    let responses = futures::future::join_all(all_futures).await;

    for res in &responses {
        assert!(
            res.status().is_success(),
            "all concurrent requests should succeed, got {}",
            res.status()
        );
    }

    // After everything settles, the list must be consistent.
    let final_list = voter_list(&app, cookie).await;
    let voters = parse_response_body::<Vec<VoterInfo>>(final_list).await;
    assert_eq!(voters.len(), 11, "10 voters + 1 host");
}

/// Concurrent close-meeting attempts: exactly one should succeed (200).
///
/// The auth extractor validates the meeting exists before passing the request to the
/// handler. After the first close-meeting removes the meeting, any subsequent auth check
/// fails with 401 (implicit JWT revocation). Depending on scheduling some concurrent
/// requests may reach the handler body while the meeting is still present but find it
/// already removed (404). We only pin down the guarantee that matters: exactly one 200.
#[tokio::test]
async fn test_concurrent_close_meeting() {
    let app = MockApp::new_inprocess();

    let creation_res = create_meeting(&app).await;
    let cookie = extract_cookie(&creation_res).1;

    let futures: Vec<_> = (0..5)
        .map(|_| {
            app.oneshot(json_request(
                Method::DELETE,
                "/api/host/close-meeting",
                serde_json::to_value(()).unwrap(),
                Some(cookie.clone()),
            ))
        })
        .collect();

    let responses = futures::future::join_all(futures).await;

    let successes = responses
        .iter()
        .filter(|r| r.status() == StatusCode::OK)
        .count();

    assert_eq!(successes, 1, "exactly one close should succeed");

    // All other responses must be non-200 (either 401 — auth revoked — or 404 —
    // handler reached the missing meeting). Both are correct depending on scheduling.
    for res in responses.iter().filter(|r| r.status() != StatusCode::OK) {
        assert!(
            res.status() == StatusCode::UNAUTHORIZED || res.status() == StatusCode::NOT_FOUND,
            "expected 401 or 404, got {}",
            res.status()
        );
    }
}

/// Concurrent remove-all calls are safe: all should return 200 and the final voter
/// list must contain only the host.
#[tokio::test]
async fn test_concurrent_remove_all() {
    let app = MockApp::new_inprocess();

    let creation_res = create_meeting(&app).await;
    let cookie = extract_cookie(&creation_res).1;

    // Add some voters first.
    for i in 0..5 {
        app.oneshot(json_request(
            Method::POST,
            "/api/host/new-voter",
            serde_json::to_value(NewVoterRequestBody {
                voter_name: format!("Voter{i}"),
                is_host: false,
            })
            .unwrap(),
            Some(cookie.clone()),
        ))
        .await;
    }

    // Issue 5 concurrent remove-all requests.
    let futures: Vec<_> = (0..5)
        .map(|_| {
            app.oneshot(json_request(
                Method::DELETE,
                "/api/host/remove-all",
                serde_json::to_value(()).unwrap(),
                Some(cookie.clone()),
            ))
        })
        .collect();

    let responses = futures::future::join_all(futures).await;
    for res in &responses {
        assert_eq!(res.status(), StatusCode::OK);
    }

    let list_res = voter_list(&app, cookie).await;
    let voters = parse_response_body::<Vec<VoterInfo>>(list_res).await;
    assert_eq!(voters.len(), 1);
    assert!(voters[0].is_host);
}
