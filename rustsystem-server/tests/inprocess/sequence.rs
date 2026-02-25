use axum::http::StatusCode;

use crate::{
    common::MockApp,
    inprocess::{
        add_voter, create_meeting, end_vote_round, extract_cookie, start_vote, tally, voter_login,
    },
};

// For this test, it should be noted that end-vote-round is always valid (it's a hard reset)
#[tokio::test]
#[ignore = "requires a running trustauth service (start-vote calls start_round_on_trustauth)"]
async fn test_lock_sequence() {
    let app = MockApp::new_inprocess();

    let creation_res = create_meeting(&app).await;

    let cookie = extract_cookie(&creation_res).1;

    for _ in 0..10 {
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

        // We should now be back to normal.
    }
}

#[tokio::test]
#[ignore = "requires a running trustauth service (start-vote calls start_round_on_trustauth)"]
async fn test_invite_lock() {
    let app = MockApp::new_inprocess();

    let creation_res = create_meeting(&app).await;

    let cookie = extract_cookie(&creation_res).1;

    // This is fine because there is no active vote
    let add_res = add_voter(&app, cookie, format!("Voter1"), false).await;
    assert_eq!(add_res.status(), StatusCode::CREATED);
    let login_res = voter_login(&app, add_res).await;
    assert_eq!(login_res.status(), StatusCode::ACCEPTED);

    start_vote(&app, cookie, vec![], 0).await;

    // This is not fine. Voter can't be added while vote is active
    let add_res = add_voter(&app, cookie, format!("Voter2"), false).await;
    assert_eq!(add_res.status(), StatusCode::CONFLICT);

    tally(&app, cookie).await;

    // Now this is fine. Voter can be added during tally phase
    let add_res = add_voter(&app, cookie, format!("Voter3"), false).await;
    assert_eq!(add_res.status(), StatusCode::CREATED);

    end_vote_round(&app, cookie).await;

    // Now we can add more voters
    let add_res = add_voter(&app, cookie, format!("Voter4"), false).await;
    assert_eq!(add_res.status(), StatusCode::CREATED);
}

#[tokio::test]
#[ignore = "requires a running trustauth service (start-vote calls start_round_on_trustauth)"]
async fn test_pending_invite_purge() {
    let app = MockApp::new_inprocess();

    let creation_res = create_meeting(&app).await;

    let cookie = extract_cookie(&creation_res).1;

    let add_res = add_voter(&app, cookie, format!("Voter1"), false).await;
    assert_eq!(add_res.status(), StatusCode::CREATED);

    // Starting the vote round purges all pending unclaimed users
    start_vote(&app, cookie, vec![], 0).await;

    // We should not be able to find the UUuid of the user that's trying to login after the vote
    // has started
    let login_res = voter_login(&app, add_res).await;
    assert_eq!(login_res.status(), StatusCode::NOT_FOUND);
}
