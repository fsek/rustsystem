/// Tests for the /api/common/* endpoints.
use axum::http::StatusCode;

use crate::{
    common::MockApp,
    inprocess::{
        add_voter, create_meeting, extract_cookie, meeting_specs, parse_response_body,
        vote_progress, voter_login,
    },
};

/// vote-progress in the initial Creation state returns sensible defaults:
/// isActive=false, isTally=false, totalVotesCast=0.
#[tokio::test]
async fn test_vote_progress_creation_state() {
    let app = MockApp::new_inprocess();

    let creation_res = create_meeting(&app).await;
    let cookie = extract_cookie(&creation_res).1;

    let res = vote_progress(&app, cookie).await;
    assert_eq!(res.status(), StatusCode::OK);

    let body = parse_response_body::<serde_json::Value>(res).await;
    assert_eq!(body["isActive"], false);
    assert_eq!(body["isTally"], false);
    assert_eq!(body["totalVotesCast"], 0);
    assert!(body["voteName"].is_null());
    assert!(body["metadata"].is_null());
}

/// vote-progress reflects the current participant count (all voters in the meeting).
#[tokio::test]
async fn test_vote_progress_participant_count() {
    let app = MockApp::new_inprocess();

    let creation_res = create_meeting(&app).await;
    let cookie = extract_cookie(&creation_res).1;

    // Only host at first.
    let res = vote_progress(&app, cookie).await;
    let body = parse_response_body::<serde_json::Value>(res).await;
    assert_eq!(body["totalParticipants"], 1);

    // Add and log in one voter.
    let add_res = add_voter(&app, cookie, "Voter1", false).await;
    voter_login(&app, add_res).await;

    let res = vote_progress(&app, cookie).await;
    let body = parse_response_body::<serde_json::Value>(res).await;
    assert_eq!(body["totalParticipants"], 2);
}

/// meeting-specs returns the correct meeting title immediately after creation.
#[tokio::test]
async fn test_meeting_specs_title() {
    let app = MockApp::new_inprocess();

    let creation_res = create_meeting(&app).await;
    let cookie = extract_cookie(&creation_res).1;

    let res = meeting_specs(&app, cookie).await;
    assert_eq!(res.status(), StatusCode::OK);

    let body = parse_response_body::<serde_json::Value>(res).await;
    assert_eq!(body["title"], "Test Meeting");
}

/// meeting-specs counts only logged-in voters as participants.
/// A voter who has been created but not yet logged in is not counted.
#[tokio::test]
async fn test_meeting_specs_participant_count() {
    let app = MockApp::new_inprocess();

    let creation_res = create_meeting(&app).await;
    let cookie = extract_cookie(&creation_res).1;

    // Host only: 1 logged-in participant.
    let res = meeting_specs(&app, cookie).await;
    let body = parse_response_body::<serde_json::Value>(res).await;
    assert_eq!(body["participants"], 1);

    // Add but don't log in → still 1 logged-in participant.
    add_voter(&app, cookie, "PendingVoter", false).await;
    let res = meeting_specs(&app, cookie).await;
    let body = parse_response_body::<serde_json::Value>(res).await;
    assert_eq!(body["participants"], 1);

    // Add and log in → now 2 logged-in participants.
    let add_res = add_voter(&app, cookie, "LoggedInVoter", false).await;
    voter_login(&app, add_res).await;
    let res = meeting_specs(&app, cookie).await;
    let body = parse_response_body::<serde_json::Value>(res).await;
    assert_eq!(body["participants"], 2);
}

/// vote-progress totalParticipants counts all voters (logged in or not),
/// while meeting-specs participants counts only logged-in voters.
#[tokio::test]
async fn test_progress_vs_specs_participant_semantics() {
    let app = MockApp::new_inprocess();

    let creation_res = create_meeting(&app).await;
    let cookie = extract_cookie(&creation_res).1;

    // Add a voter but do not log them in.
    add_voter(&app, cookie, "Pending", false).await;

    let progress_res = vote_progress(&app, cookie).await;
    let progress = parse_response_body::<serde_json::Value>(progress_res).await;

    let specs_res = meeting_specs(&app, cookie).await;
    let specs = parse_response_body::<serde_json::Value>(specs_res).await;

    // vote-progress counts everyone in the map (host + pending voter = 2).
    assert_eq!(progress["totalParticipants"], 2);
    // meeting-specs counts only logged-in voters (host only = 1).
    assert_eq!(specs["participants"], 1);
}
