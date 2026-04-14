/// Tests verifying that server-side field length limits are enforced correctly.
///
/// For each validated field we run two cases:
///   - exactly at the limit  → request should proceed past validation
///   - one character over    → 422 Unprocessable Entity
///
/// `start_vote` tests that are at the limit will return 500 rather than 200
/// because no trustauth service is running in inprocess tests (the request
/// fails at the trustauth call, after field validation has already passed).
use axum::http::{HeaderValue, Method, StatusCode};
use axum::response::Response;

use rustsystem_core::{MAX_LABEL_LENGTH, MAX_NAME_LENGTH};
use rustsystem_server::{
    api::{
        create_meeting::CreateMeetingRequest,
        host::{new_voter::NewVoterRequestBody, start_vote::StartVoteRequest},
    },
    proof::BallotMetaData,
};

use crate::common::{MockApp, json_request};
use crate::inprocess::extract_cookie;

// ── Local helpers ──────────────────────────────────────────────────────────────

async fn create_meeting_custom(app: &MockApp, title: String, host_name: String) -> Response {
    app.oneshot(json_request(
        Method::POST,
        "/api/create-meeting",
        serde_json::to_value(CreateMeetingRequest {
            title,
            host_name,
            pub_key: String::new(),
        })
        .unwrap(),
        None,
    ))
    .await
}

async fn add_voter_named(app: &MockApp, cookie: &HeaderValue, name: String) -> Response {
    app.oneshot(json_request(
        Method::POST,
        "/api/host/new-voter",
        serde_json::to_value(NewVoterRequestBody {
            voter_name: name,
            is_host: false,
        })
        .unwrap(),
        Some(cookie.clone()),
    ))
    .await
}

async fn start_vote_named(app: &MockApp, cookie: &HeaderValue, name: String) -> Response {
    app.oneshot(json_request(
        Method::POST,
        "/api/host/start-vote",
        serde_json::to_value(StartVoteRequest {
            name,
            shuffle: false,
            metadata: BallotMetaData::new(vec!["A".into(), "B".into()], 1, 1),
        })
        .unwrap(),
        Some(cookie.clone()),
    ))
    .await
}

async fn start_vote_with_candidates(
    app: &MockApp,
    cookie: &HeaderValue,
    candidates: Vec<String>,
) -> Response {
    app.oneshot(json_request(
        Method::POST,
        "/api/host/start-vote",
        serde_json::to_value(StartVoteRequest {
            name: String::from("Test Vote"),
            shuffle: false,
            metadata: BallotMetaData::new(candidates, 1, 1),
        })
        .unwrap(),
        Some(cookie.clone()),
    ))
    .await
}

// ── Meeting title ──────────────────────────────────────────────────────────────

#[tokio::test]
async fn test_create_meeting_title_at_limit() {
    let app = MockApp::new_inprocess();
    let res = create_meeting_custom(&app, "x".repeat(MAX_LABEL_LENGTH), "Host".into()).await;
    assert_eq!(res.status(), StatusCode::CREATED);
}

#[tokio::test]
async fn test_create_meeting_title_over_limit() {
    let app = MockApp::new_inprocess();
    let res =
        create_meeting_custom(&app, "x".repeat(MAX_LABEL_LENGTH + 1), "Host".into()).await;
    assert_eq!(res.status(), StatusCode::UNPROCESSABLE_ENTITY);
}

// ── Meeting host name ──────────────────────────────────────────────────────────

#[tokio::test]
async fn test_create_meeting_host_name_at_limit() {
    let app = MockApp::new_inprocess();
    let res =
        create_meeting_custom(&app, "Test Meeting".into(), "x".repeat(MAX_NAME_LENGTH)).await;
    assert_eq!(res.status(), StatusCode::CREATED);
}

#[tokio::test]
async fn test_create_meeting_host_name_over_limit() {
    let app = MockApp::new_inprocess();
    let res =
        create_meeting_custom(&app, "Test Meeting".into(), "x".repeat(MAX_NAME_LENGTH + 1))
            .await;
    assert_eq!(res.status(), StatusCode::UNPROCESSABLE_ENTITY);
}

// ── Voter name ─────────────────────────────────────────────────────────────────

#[tokio::test]
async fn test_add_voter_name_at_limit() {
    let app = MockApp::new_inprocess();
    let creation_res = create_meeting_custom(&app, "Meeting".into(), "Host".into()).await;
    let cookie = extract_cookie(&creation_res).1;

    let res = add_voter_named(&app, cookie, "x".repeat(MAX_NAME_LENGTH)).await;
    assert_eq!(res.status(), StatusCode::CREATED);
}

#[tokio::test]
async fn test_add_voter_name_over_limit() {
    let app = MockApp::new_inprocess();
    let creation_res = create_meeting_custom(&app, "Meeting".into(), "Host".into()).await;
    let cookie = extract_cookie(&creation_res).1;

    let res = add_voter_named(&app, cookie, "x".repeat(MAX_NAME_LENGTH + 1)).await;
    assert_eq!(res.status(), StatusCode::UNPROCESSABLE_ENTITY);
}

// ── Vote round name ────────────────────────────────────────────────────────────

#[tokio::test]
async fn test_start_vote_round_name_at_limit() {
    // Field validation passes; the request then fails at the trustauth call
    // (no trustauth in inprocess tests) → 500, not 422.
    let app = MockApp::new_inprocess();
    let creation_res = create_meeting_custom(&app, "Meeting".into(), "Host".into()).await;
    let cookie = extract_cookie(&creation_res).1;

    let res = start_vote_named(&app, cookie, "x".repeat(MAX_LABEL_LENGTH)).await;
    assert_ne!(res.status(), StatusCode::UNPROCESSABLE_ENTITY);
}

#[tokio::test]
async fn test_start_vote_round_name_over_limit() {
    let app = MockApp::new_inprocess();
    let creation_res = create_meeting_custom(&app, "Meeting".into(), "Host".into()).await;
    let cookie = extract_cookie(&creation_res).1;

    let res = start_vote_named(&app, cookie, "x".repeat(MAX_LABEL_LENGTH + 1)).await;
    assert_eq!(res.status(), StatusCode::UNPROCESSABLE_ENTITY);
}

// ── Candidate name ─────────────────────────────────────────────────────────────

#[tokio::test]
async fn test_start_vote_candidate_name_at_limit() {
    // Field validation passes; the request then fails at the trustauth call → 500.
    let app = MockApp::new_inprocess();
    let creation_res = create_meeting_custom(&app, "Meeting".into(), "Host".into()).await;
    let cookie = extract_cookie(&creation_res).1;

    let long_name = "x".repeat(MAX_NAME_LENGTH);
    let res =
        start_vote_with_candidates(&app, cookie, vec![long_name, "Other".into()]).await;
    assert_ne!(res.status(), StatusCode::UNPROCESSABLE_ENTITY);
}

#[tokio::test]
async fn test_start_vote_candidate_name_over_limit() {
    let app = MockApp::new_inprocess();
    let creation_res = create_meeting_custom(&app, "Meeting".into(), "Host".into()).await;
    let cookie = extract_cookie(&creation_res).1;

    let too_long = "x".repeat(MAX_NAME_LENGTH + 1);
    let res =
        start_vote_with_candidates(&app, cookie, vec![too_long, "Other".into()]).await;
    assert_eq!(res.status(), StatusCode::UNPROCESSABLE_ENTITY);
}
