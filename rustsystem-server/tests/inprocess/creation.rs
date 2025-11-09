use axum::http::{Method, StatusCode};

use crate::{
    common::{MockApp, json_request},
    inprocess::{add_voter, clone_response, create_meeting, extract_cookie, voter_login},
};

use rustsystem_server::api::create_meeting::CreateMeetingRequest;

/// create-meeting -> 201
#[tokio::test]
async fn test_meeting_creation() {
    let app = MockApp::new_inprocess();

    let title = String::from("Test Meeting");
    let host_name = String::from("Creator");

    let res = app
        .oneshot(json_request(
            Method::POST,
            "/api/create-meeting",
            serde_json::to_value(CreateMeetingRequest { title, host_name }).unwrap(),
            None,
        ))
        .await;

    assert_eq!(res.status(), StatusCode::CREATED);

    assert!(
        res.headers()
            .iter()
            .find(|(name, _value)| name.as_str() == "set-cookie")
            .is_some()
    );
}

/// create-meeting -> 201
/// vote-active -> 401
#[tokio::test]
async fn test_simple_auth_reject() {
    let app = MockApp::new_inprocess();
    create_meeting(&app).await;
    let res = app
        .oneshot(json_request(
            Method::GET,
            "/api/common/vote-active",
            serde_json::to_value(()).unwrap(),
            None,
        ))
        .await;

    assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
}

/// create-meeting -> 201
/// vote-active -> 200
#[tokio::test]
async fn test_simple_auth_accept() {
    let app = MockApp::new_inprocess();

    let creation_res = create_meeting(&app).await;

    let cookie = extract_cookie(&creation_res).1;

    let res = app
        .oneshot(json_request(
            Method::GET,
            "/api/common/vote-active",
            serde_json::to_value(()).unwrap(),
            Some(cookie.clone()),
        ))
        .await;

    assert_eq!(res.status(), StatusCode::OK);
}

/// create-meeting -> 201
/// new-voter -> 201
/// login -> 202
/// login -> 409
/// new-voter -> 409
#[tokio::test]
async fn test_new_voter_login() {
    let app = MockApp::new_inprocess();

    let creation_res = create_meeting(&app).await;

    let cookie = extract_cookie(&creation_res).1;

    let add_res = add_voter(&app, cookie, "Voter1", false).await;
    assert_eq!(add_res.status(), StatusCode::CREATED);

    let (add_res1, add_res2) = clone_response(add_res).await;
    let login_res1 = voter_login(&app, add_res1).await;
    assert_eq!(login_res1.status(), StatusCode::ACCEPTED);

    let login_res2 = voter_login(&app, add_res2).await;
    assert_eq!(login_res2.status(), StatusCode::CONFLICT);

    // Try to add voter with same name
    let add_res = add_voter(&app, cookie, "Voter1", false).await;
    assert_eq!(add_res.status(), StatusCode::CONFLICT);
}

/// create-meeting -> 201
/// (
/// new-voter -> 201
/// login -> 202
/// )x10
#[tokio::test]
async fn test_many_voter_login() {
    let app = MockApp::new_inprocess();

    let creation_res = create_meeting(&app).await;

    let cookie = extract_cookie(&creation_res).1;

    for i in 0..10 {
        let add_res = add_voter(&app, cookie, format!("Voter{i}"), false).await;
        assert_eq!(add_res.status(), StatusCode::CREATED);
        let login_res = voter_login(&app, add_res).await;
        assert_eq!(login_res.status(), StatusCode::ACCEPTED);
    }
}
