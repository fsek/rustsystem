/// Negative / edge-case tests for individual endpoints.
use axum::http::StatusCode;
use uuid::Uuid;

use crate::{
    common::MockApp,
    inprocess::{
        add_voter, close_meeting, create_meeting, extract_cookie, parse_response_body,
        remove_voter, reset_login, voter_id, voter_list, voter_login,
    },
};
use rustsystem_server::api::host::voter_list::VoterInfo;

/// voter-id returns 404 when asked for a name that does not exist.
#[tokio::test]
async fn test_voter_id_not_found() {
    let app = MockApp::new_inprocess();

    let creation_res = create_meeting(&app).await;
    let cookie = extract_cookie(&creation_res).1;

    let res = voter_id(&app, cookie, "NoSuchPerson".into()).await;
    assert_eq!(res.status(), StatusCode::NOT_FOUND);
}

/// reset-login returns 404 when given a UUID that is not in the meeting.
#[tokio::test]
async fn test_reset_login_not_found() {
    let app = MockApp::new_inprocess();

    let creation_res = create_meeting(&app).await;
    let cookie = extract_cookie(&creation_res).1;

    let res = reset_login(&app, cookie, Uuid::new_v4()).await;
    assert_eq!(res.status(), StatusCode::NOT_FOUND);
}

/// Adding a voter with a name that already exists returns 409 Conflict.
#[tokio::test]
async fn test_add_voter_duplicate_name() {
    let app = MockApp::new_inprocess();

    let creation_res = create_meeting(&app).await;
    let cookie = extract_cookie(&creation_res).1;

    let res1 = add_voter(&app, cookie, "Alice", false).await;
    assert_eq!(res1.status(), StatusCode::CREATED);

    let res2 = add_voter(&app, cookie, "Alice", false).await;
    assert_eq!(res2.status(), StatusCode::CONFLICT);
}

/// A voter cannot log in twice with the same invite link (UUID already claimed).
#[tokio::test]
async fn test_double_login_rejected() {
    let app = MockApp::new_inprocess();

    let creation_res = create_meeting(&app).await;
    let cookie = extract_cookie(&creation_res).1;

    let add_res = add_voter(&app, cookie, "Bob", false).await;

    let (clone1, clone2) = {
        use axum::body::Body;
        use http_body_util::BodyExt;
        let (parts, body) = add_res.into_parts();
        let bytes = body.collect().await.unwrap().to_bytes();
        let r1 = axum::response::Response::from_parts(parts.clone(), Body::from(bytes.clone()));
        let r2 = axum::response::Response::from_parts(parts, Body::from(bytes));
        (r1, r2)
    };

    let login1 = voter_login(&app, clone1).await;
    assert_eq!(login1.status(), StatusCode::ACCEPTED);

    let login2 = voter_login(&app, clone2).await;
    assert_eq!(login2.status(), StatusCode::CONFLICT);
}

/// remove-voter with a UUID that does not belong to the meeting returns 404.
#[tokio::test]
async fn test_remove_voter_not_found() {
    let app = MockApp::new_inprocess();

    let creation_res = create_meeting(&app).await;
    let cookie = extract_cookie(&creation_res).1;

    let res = remove_voter(&app, cookie, Uuid::new_v4()).await;
    assert_eq!(res.status(), StatusCode::NOT_FOUND);
}

/// Removing a voter and then searching for their name returns 404.
#[tokio::test]
async fn test_voter_id_after_removal() {
    let app = MockApp::new_inprocess();

    let creation_res = create_meeting(&app).await;
    let cookie = extract_cookie(&creation_res).1;

    let add_res = add_voter(&app, cookie, "Charlie", false).await;
    voter_login(&app, add_res).await;

    let id_res = voter_id(&app, cookie, "Charlie".into()).await;
    let id = parse_response_body::<Uuid>(id_res).await;

    let remove_res = remove_voter(&app, cookie, id).await;
    assert_eq!(remove_res.status(), StatusCode::OK);

    let id_res2 = voter_id(&app, cookie, "Charlie".into()).await;
    assert_eq!(id_res2.status(), StatusCode::NOT_FOUND);
}

/// After close-meeting the host cookie is implicitly revoked: any subsequent request
/// is rejected with 401 because the auth extractor validates that both the meeting and
/// the voter still exist, and neither do once the meeting is deleted.
#[tokio::test]
async fn test_cookie_revoked_after_close() {
    let app = MockApp::new_inprocess();

    let creation_res = create_meeting(&app).await;
    let cookie = extract_cookie(&creation_res).1;

    let close_res = close_meeting(&app, cookie).await;
    assert_eq!(close_res.status(), StatusCode::OK);

    let list_res = voter_list(&app, cookie).await;
    assert_eq!(list_res.status(), StatusCode::UNAUTHORIZED);

    let id_res = voter_id(&app, cookie, "Creator".into()).await;
    assert_eq!(id_res.status(), StatusCode::UNAUTHORIZED);
}

/// The host's own name is always in the voter list after meeting creation.
#[tokio::test]
async fn test_host_always_in_voter_list() {
    let app = MockApp::new_inprocess();

    let creation_res = create_meeting(&app).await;
    let cookie = extract_cookie(&creation_res).1;

    let list_res = voter_list(&app, cookie).await;
    let voters = parse_response_body::<Vec<VoterInfo>>(list_res).await;

    assert_eq!(voters.len(), 1);
    let host = &voters[0];
    assert_eq!(host.name, "Creator");
    assert!(host.is_host);
    assert!(host.logged_in);
}
