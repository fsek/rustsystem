/// State-machine tests that do not require a running trustauth service.
///
/// The tests that advance the state through Voting (start-vote → tally → end-round)
/// are in sequence.rs and still require trustauth. These tests exercise only the
/// transitions and rejections that are reachable from the initial Creation state.
use axum::http::StatusCode;

use crate::{
    common::MockApp,
    inprocess::{create_meeting, end_vote_round, extract_cookie, get_tally, tally},
};

/// Calling tally while the meeting is in Creation state (no active vote round) must
/// return 410 Gone (VotingInactive).
#[tokio::test]
async fn test_tally_requires_active_vote() {
    let app = MockApp::new_inprocess();

    let creation_res = create_meeting(&app).await;
    let cookie = extract_cookie(&creation_res).1;

    let tally_res = tally(&app, cookie).await;
    assert_eq!(tally_res.status(), StatusCode::GONE);
}

/// Calling tally twice from Creation state keeps returning 410 Gone.
#[tokio::test]
async fn test_tally_always_fails_in_creation_state() {
    let app = MockApp::new_inprocess();

    let creation_res = create_meeting(&app).await;
    let cookie = extract_cookie(&creation_res).1;

    for _ in 0..3 {
        let tally_res = tally(&app, cookie).await;
        assert_eq!(tally_res.status(), StatusCode::GONE);
    }
}

/// get-tally before any tally has been computed returns 409 Conflict (InvalidState),
/// because there is no last_tally to read.
#[tokio::test]
async fn test_get_tally_before_any_tally() {
    let app = MockApp::new_inprocess();

    let creation_res = create_meeting(&app).await;
    let cookie = extract_cookie(&creation_res).1;

    let res = get_tally(&app, cookie).await;
    assert_eq!(res.status(), StatusCode::CONFLICT);
}

/// end-vote-round succeeds from Creation state — it is a hard reset with no state
/// precondition. Subsequent tally calls still return 410.
#[tokio::test]
async fn test_end_vote_round_then_tally_still_fails() {
    let app = MockApp::new_inprocess();

    let creation_res = create_meeting(&app).await;
    let cookie = extract_cookie(&creation_res).1;

    let reset_res = end_vote_round(&app, cookie).await;
    assert_eq!(reset_res.status(), StatusCode::OK);

    let tally_res = tally(&app, cookie).await;
    assert_eq!(tally_res.status(), StatusCode::GONE);
}
