use axum::http::StatusCode;

use crate::{
    common::MockApp,
    inprocess::{
        add_voter, create_meeting, end_vote_round, extract_cookie, start_vote, tally, voter_login,
    },
};

#[tokio::test]
#[ignore = "requires a running trustauth service (start-vote calls start_round_on_trustauth)"]
async fn test_state_permissions() {
    let app = MockApp::new_inprocess();

    let creation_res = create_meeting(&app).await;

    let host_cookie = extract_cookie(&creation_res).1;

    let add_res = add_voter(&app, host_cookie, format!("Voter1"), false).await;
    let login_res = voter_login(&app, add_res).await;

    let voter_cookie = extract_cookie(&login_res).1;

    let start_res_unauthorized = start_vote(&app, voter_cookie, vec![], 0).await;
    assert_eq!(start_res_unauthorized.status(), StatusCode::UNAUTHORIZED);
    let start_res_authorized = start_vote(&app, host_cookie, vec![], 0).await;
    assert_eq!(start_res_authorized.status(), StatusCode::OK);

    let tally_res_unauthorized = tally(&app, voter_cookie).await;
    assert_eq!(tally_res_unauthorized.status(), StatusCode::UNAUTHORIZED);
    let tally_res_authorized = tally(&app, host_cookie).await;
    assert_eq!(tally_res_authorized.status(), StatusCode::OK);

    let reset_res_unauthorized = end_vote_round(&app, voter_cookie).await;
    assert_eq!(reset_res_unauthorized.status(), StatusCode::UNAUTHORIZED);
    let reset_res_authorized = end_vote_round(&app, host_cookie).await;
    assert_eq!(reset_res_authorized.status(), StatusCode::OK);
}
