/// Meeting lifecycle tests: close-meeting, remove-all, end-vote-round, get-all-tally.
use axum::http::StatusCode;

use crate::{
    common::MockApp,
    inprocess::{
        add_voter, close_meeting, create_meeting, end_vote_round, extract_cookie,
        get_all_tally, parse_response_body, remove_all, voter_id, voter_list, voter_login,
    },
};
use rustsystem_server::api::host::voter_list::VoterInfo;

/// close-meeting returns 200, after which requests using the same cookie return 404 because
/// the meeting no longer exists in the server's map.
#[tokio::test]
async fn test_close_meeting_success() {
    let app = MockApp::new_inprocess();

    let creation_res = create_meeting(&app).await;
    let cookie = extract_cookie(&creation_res).1;

    let close_res = close_meeting(&app, cookie).await;
    assert_eq!(close_res.status(), StatusCode::OK);

    // The auth extractor validates that the meeting still exists before passing the
    // request through. Once the meeting is deleted the JWT is implicitly revoked,
    // so any subsequent request with that cookie is rejected with 401.
    let list_res = voter_list(&app, cookie).await;
    assert_eq!(list_res.status(), StatusCode::UNAUTHORIZED);
}

/// remove-all removes every non-host voter; the host remains.
#[tokio::test]
async fn test_remove_all_keeps_host() {
    let app = MockApp::new_inprocess();

    let creation_res = create_meeting(&app).await;
    let cookie = extract_cookie(&creation_res).1;

    for i in 0..5 {
        let add_res = add_voter(&app, cookie, format!("Voter{i}"), false).await;
        voter_login(&app, add_res).await;
    }

    let list_res = voter_list(&app, cookie).await;
    let voters = parse_response_body::<Vec<VoterInfo>>(list_res).await;
    assert_eq!(voters.len(), 6); // 5 voters + host

    let remove_res = remove_all(&app, cookie).await;
    assert_eq!(remove_res.status(), StatusCode::OK);

    let list_res = voter_list(&app, cookie).await;
    let voters = parse_response_body::<Vec<VoterInfo>>(list_res).await;
    assert_eq!(voters.len(), 1);
    assert_eq!(voters[0].name, "Creator");
    assert!(voters[0].is_host);
}

/// remove-all with no non-host voters is still OK.
#[tokio::test]
async fn test_remove_all_empty() {
    let app = MockApp::new_inprocess();

    let creation_res = create_meeting(&app).await;
    let cookie = extract_cookie(&creation_res).1;

    let remove_res = remove_all(&app, cookie).await;
    assert_eq!(remove_res.status(), StatusCode::OK);

    let list_res = voter_list(&app, cookie).await;
    let voters = parse_response_body::<Vec<VoterInfo>>(list_res).await;
    assert_eq!(voters.len(), 1);
}

/// end-vote-round is a hard reset and succeeds even when called in the Creation state
/// (i.e. when no vote is active). Calling it multiple times is also fine.
#[tokio::test]
async fn test_end_vote_round_idempotent() {
    let app = MockApp::new_inprocess();

    let creation_res = create_meeting(&app).await;
    let cookie = extract_cookie(&creation_res).1;

    for _ in 0..3 {
        let res = end_vote_round(&app, cookie).await;
        assert_eq!(res.status(), StatusCode::OK);
    }
}

/// get-all-tally returns an empty array when no tally files have been written yet.
#[tokio::test]
async fn test_get_all_tally_empty() {
    let app = MockApp::new_inprocess();

    let creation_res = create_meeting(&app).await;
    let cookie = extract_cookie(&creation_res).1;

    let res = get_all_tally(&app, cookie).await;
    assert_eq!(res.status(), StatusCode::OK);

    let files = parse_response_body::<Vec<serde_json::Value>>(res).await;
    assert!(files.is_empty());
}

/// remove-all followed by adding new voters works correctly: the meeting remains open.
#[tokio::test]
async fn test_remove_all_then_add() {
    let app = MockApp::new_inprocess();

    let creation_res = create_meeting(&app).await;
    let cookie = extract_cookie(&creation_res).1;

    let add_res = add_voter(&app, cookie, "Voter1", false).await;
    assert_eq!(add_res.status(), StatusCode::CREATED);

    let remove_res = remove_all(&app, cookie).await;
    assert_eq!(remove_res.status(), StatusCode::OK);

    // Voter1 was removed, so adding with the same name should succeed.
    let add_res2 = add_voter(&app, cookie, "Voter1", false).await;
    assert_eq!(add_res2.status(), StatusCode::CREATED);

    let id_res = voter_id(&app, cookie, "Creator".into()).await;
    assert_eq!(id_res.status(), StatusCode::OK);
}
