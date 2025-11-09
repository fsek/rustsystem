use axum::http::StatusCode;

use crate::{
    common::MockApp,
    inprocess::{create_meeting, end_vote_round, extract_cookie, start_vote, tally},
};

// For this test, it should be noted that end-vote-round is always valid (it's a hard reset)
#[tokio::test]
async fn test_lock_sequence() {
    let app = MockApp::new_inprocess();

    let creation_res = create_meeting(&app).await;

    let cookie = extract_cookie(&creation_res).1;

    // Neutral state. Only start vote.

    let tally_res = tally(&app, cookie).await;
    assert_eq!(tally_res.status(), StatusCode::GONE);

    let start_vote_res = start_vote(&app, cookie, vec![], 0).await;
    assert_eq!(start_vote_res.status(), StatusCode::OK);

    // Voting Active. Only tally is valid in this state.

    let start_vote_res = start_vote(&app, cookie, vec![], 0).await;
    assert_eq!(start_vote_res.status(), StatusCode::CONFLICT);

    let tally_res = tally(&app, cookie).await;
    assert_eq!(tally_res.status(), StatusCode::OK);

    // Tally active. Nothing is valid. Requires reset.

    let start_vote_res = start_vote(&app, cookie, vec![], 0).await;
    assert_eq!(start_vote_res.status(), StatusCode::CONFLICT);

    let tally_res = tally(&app, cookie).await;
    assert_eq!(tally_res.status(), StatusCode::GONE);

    let end_vote_res = end_vote_round(&app, cookie).await;
    assert_eq!(end_vote_res.status(), StatusCode::OK);

    // We should now be back to normal

    let tally_res = tally(&app, cookie).await;
    assert_eq!(tally_res.status(), StatusCode::GONE);

    let start_vote_res = start_vote(&app, cookie, vec![], 0).await;
    assert_eq!(start_vote_res.status(), StatusCode::OK);
}
