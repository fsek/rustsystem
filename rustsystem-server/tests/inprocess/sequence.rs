use axum::http::StatusCode;

use crate::{
    common::MockApp,
    inprocess::{create_meeting, end_vote_round, extract_cookie, lock, start_vote, tally, unlock},
};

// For this test, it should be noted that end-vote-round is always valid (it's a hard reset)
#[tokio::test]
async fn test_lock_sequence() {
    let app = MockApp::new_inprocess();

    let creation_res = create_meeting(&app).await;

    let cookie = extract_cookie(&creation_res).1;

    // Neutral state. Only lock is valid.
    let unlock_res = unlock(&app, cookie).await;
    assert_eq!(unlock_res.status(), StatusCode::CONFLICT);

    let start_vote_res = start_vote(&app, cookie).await;
    assert_eq!(start_vote_res.status(), StatusCode::CONFLICT);

    let tally_res = tally(&app, cookie).await;
    assert_eq!(tally_res.status(), StatusCode::GONE);

    let lock_res = lock(&app, cookie).await;
    assert_eq!(lock_res.status(), StatusCode::OK);

    // Locked state. Only start vote and unlock is valid

    let lock_res = lock(&app, cookie).await;
    assert_eq!(lock_res.status(), StatusCode::CONFLICT);

    let tally_res = tally(&app, cookie).await;
    assert_eq!(tally_res.status(), StatusCode::GONE);

    let start_vote_res = start_vote(&app, cookie).await;
    assert_eq!(start_vote_res.status(), StatusCode::OK);

    // Voting Active. Only tally is valid in this state.

    let lock_res = lock(&app, cookie).await;
    assert_eq!(lock_res.status(), StatusCode::CONFLICT);

    let unlock_res = unlock(&app, cookie).await;
    assert_eq!(unlock_res.status(), StatusCode::CONFLICT);

    let start_vote_res = start_vote(&app, cookie).await;
    assert_eq!(start_vote_res.status(), StatusCode::CONFLICT);

    let tally_res = tally(&app, cookie).await;
    assert_eq!(tally_res.status(), StatusCode::OK);

    // Tally active. Nothing is valid. Requires reset.

    let lock_res = lock(&app, cookie).await;
    assert_eq!(lock_res.status(), StatusCode::CONFLICT);

    let unlock_res = unlock(&app, cookie).await;
    assert_eq!(unlock_res.status(), StatusCode::CONFLICT);

    let start_vote_res = start_vote(&app, cookie).await;
    assert_eq!(start_vote_res.status(), StatusCode::CONFLICT);

    let tally_res = tally(&app, cookie).await;
    assert_eq!(tally_res.status(), StatusCode::GONE);

    let end_vote_res = end_vote_round(&app, cookie).await;
    assert_eq!(end_vote_res.status(), StatusCode::OK);
}
