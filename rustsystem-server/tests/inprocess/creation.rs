use axum::{
    http::{HeaderName, HeaderValue, Method, StatusCode, header},
    response::Response,
};

use crate::{
    common::{MockApp, json_request},
    inprocess::qr_reader::{extract_url_args, read_qr},
};

use http_body_util::BodyExt;
use rustsystem_server::api::{
    create_meeting::{CreateMeetingRequest, CreateMeetingResponse},
    host::new_voter::NewVoterRequestBody,
    login::LoginRequest,
};

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

async fn create_meeting(app: &MockApp) -> Response {
    let title = String::from("Test Meeting");
    let host_name = String::from("Creator");

    app.oneshot(json_request(
        Method::POST,
        "/api/create-meeting",
        serde_json::to_value(CreateMeetingRequest { title, host_name }).unwrap(),
        None,
    ))
    .await
}

fn extract_cookie(res: &Response) -> (&HeaderName, &HeaderValue) {
    res.headers()
        .iter()
        .find(|(name, _value)| name.as_str() == "set-cookie")
        .unwrap()
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

async fn add_voter(
    app: &MockApp,
    cookie: &HeaderValue,
    name: impl Into<String>,
    is_host: bool,
) -> Response {
    app.oneshot(json_request(
        Method::POST,
        "/api/host/new-voter",
        serde_json::to_value(NewVoterRequestBody {
            voter_name: name.into(),
            is_host,
        })
        .unwrap(),
        Some(cookie.clone()),
    ))
    .await
}

async fn voter_login(app: &MockApp, res: Response) -> Response {
    let collected = res.into_body().collect().await.unwrap();
    let bytes = collected.to_bytes();
    let body_str = String::from_utf8(bytes.to_vec()).unwrap();

    let url = read_qr(&body_str).unwrap();

    let (uuuid, muuid) = extract_url_args(&url).unwrap();

    app.oneshot(json_request(
        Method::POST,
        "/api/login",
        serde_json::to_value(LoginRequest {
            uuuid: uuuid,
            muuid: muuid,
            admin_cred: None,
        })
        .unwrap(),
        None,
    ))
    .await
}

/// create-meeting -> 201
/// new-voter -> 201
/// login -> 202
#[tokio::test]
async fn test_new_voter_login() {
    let app = MockApp::new_inprocess();

    let creation_res = create_meeting(&app).await;

    let cookie = extract_cookie(&creation_res).1;

    let add_res = add_voter(&app, cookie, "Voter1", false).await;
    assert_eq!(add_res.status(), StatusCode::CREATED);

    let login_res = voter_login(&app, add_res).await;
    assert_eq!(login_res.status(), StatusCode::ACCEPTED);
}
