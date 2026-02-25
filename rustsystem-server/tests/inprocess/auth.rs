/// Authorization tests: host-only endpoints must reject voter cookies and missing cookies.
///
/// These tests do not require trustauth because the auth extractor (`AuthHost`) validates
/// the JWT and the `is_host` flag before the handler body runs, so the trustauth call in
/// `start-vote` is never reached when the request is unauthorized.
use axum::http::{Method, StatusCode};
use uuid::Uuid;

use crate::{
    common::{MockApp, json_request},
    inprocess::{
        add_voter, close_meeting, create_meeting, end_vote_round, extract_cookie,
        get_all_tally, get_tally, remove_all, reset_login, start_vote,
        tally, voter_id, voter_list, voter_login,
    },
};
use rustsystem_server::api::host::remove_voter::RemoveVoterRequest;

/// Every host-only endpoint must respond 401 when called with a plain voter (non-host) cookie.
#[tokio::test]
async fn test_voter_cookie_rejected_from_host_endpoints() {
    let app = MockApp::new_inprocess();

    let creation_res = create_meeting(&app).await;
    let host_cookie = extract_cookie(&creation_res).1;

    let add_res = add_voter(&app, host_cookie, "Voter", false).await;
    let login_res = voter_login(&app, add_res).await;
    assert_eq!(login_res.status(), StatusCode::ACCEPTED);
    let voter_cookie = extract_cookie(&login_res).1;

    // start-vote
    let res = start_vote(&app, voter_cookie, vec![], 0).await;
    assert_eq!(res.status(), StatusCode::UNAUTHORIZED);

    // tally
    let res = tally(&app, voter_cookie).await;
    assert_eq!(res.status(), StatusCode::UNAUTHORIZED);

    // end-vote-round
    let res = end_vote_round(&app, voter_cookie).await;
    assert_eq!(res.status(), StatusCode::UNAUTHORIZED);

    // close-meeting
    let res = close_meeting(&app, voter_cookie).await;
    assert_eq!(res.status(), StatusCode::UNAUTHORIZED);

    // remove-all
    let res = remove_all(&app, voter_cookie).await;
    assert_eq!(res.status(), StatusCode::UNAUTHORIZED);

    // voter-list
    let res = voter_list(&app, voter_cookie).await;
    assert_eq!(res.status(), StatusCode::UNAUTHORIZED);

    // voter-id
    let res = voter_id(&app, voter_cookie, "Creator".into()).await;
    assert_eq!(res.status(), StatusCode::UNAUTHORIZED);

    // get-tally
    let res = get_tally(&app, voter_cookie).await;
    assert_eq!(res.status(), StatusCode::UNAUTHORIZED);

    // get-all-tally
    let res = get_all_tally(&app, voter_cookie).await;
    assert_eq!(res.status(), StatusCode::UNAUTHORIZED);

    // new-voter
    let res = add_voter(&app, voter_cookie, "AnotherVoter", false).await;
    assert_eq!(res.status(), StatusCode::UNAUTHORIZED);

    // remove-voter (random UUID)
    let res = app
        .oneshot(json_request(
            Method::DELETE,
            "/api/host/remove-voter",
            serde_json::to_value(RemoveVoterRequest {
                voter_uuuid: Uuid::new_v4(),
            })
            .unwrap(),
            Some(voter_cookie.clone()),
        ))
        .await;
    assert_eq!(res.status(), StatusCode::UNAUTHORIZED);

    // reset-login (random UUID)
    let res = reset_login(&app, voter_cookie, Uuid::new_v4()).await;
    assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
}

/// Requests without any cookie must be rejected on all protected endpoints.
#[tokio::test]
async fn test_no_cookie_rejected_from_protected_endpoints() {
    let app = MockApp::new_inprocess();
    create_meeting(&app).await;

    let endpoints: &[(Method, &str)] = &[
        (Method::POST, "/api/host/start-vote"),
        (Method::POST, "/api/host/tally"),
        (Method::DELETE, "/api/host/end-vote-round"),
        (Method::DELETE, "/api/host/close-meeting"),
        (Method::DELETE, "/api/host/remove-all"),
        (Method::GET, "/api/host/voter-list"),
        (Method::GET, "/api/host/voter-id"),
        (Method::GET, "/api/host/get-tally"),
        (Method::GET, "/api/host/get-all-tally"),
        (Method::POST, "/api/host/new-voter"),
        (Method::DELETE, "/api/host/remove-voter"),
        (Method::POST, "/api/host/reset-login"),
        (Method::GET, "/api/common/vote-active"),
        (Method::GET, "/api/common/vote-progress"),
        (Method::GET, "/api/common/meeting-specs"),
    ];

    for (method, uri) in endpoints {
        let res = app
            .oneshot(json_request(
                method.clone(),
                uri,
                serde_json::to_value(()).unwrap(),
                None, // no cookie
            ))
            .await;
        assert_eq!(
            res.status(),
            StatusCode::UNAUTHORIZED,
            "{method} {uri} should require authentication"
        );
    }
}
