use axum::http::StatusCode;

use crate::{
    common::MockApp,
    inprocess::{add_voter, create_meeting, extract_cookie, lock, start_vote, unlock, voter_login},
};

/// create-meeting -> 201
/// new-voter -> 201
/// login -> 202
/// lock -> 401
/// lock -> 200
/// start-vote -> 401
/// start-vote -> 200
/// unlock -> 401
/// unlock -> 200
#[tokio::test]
async fn test_state_permissions() {
    let app = MockApp::new_inprocess();

    let creation_res = create_meeting(&app).await;

    let host_cookie = extract_cookie(&creation_res).1;

    let add_res = add_voter(&app, host_cookie, format!("Voter1"), false).await;
    let login_res = voter_login(&app, add_res).await;

    let voter_cookie = extract_cookie(&login_res).1;

    let lock_res_unauthorized = lock(&app, voter_cookie).await;
    assert_eq!(lock_res_unauthorized.status(), StatusCode::UNAUTHORIZED);
    let lock_res_authorized = lock(&app, host_cookie).await;
    assert_eq!(lock_res_authorized.status(), StatusCode::OK);

    let start_vote_res_unathorized = start_vote(&app, voter_cookie).await;
    assert_eq!(
        start_vote_res_unathorized.status(),
        StatusCode::UNAUTHORIZED
    );
    let start_vote_res_athorized = start_vote(&app, host_cookie).await;
    assert_eq!(start_vote_res_athorized.status(), StatusCode::OK);

    let unlock_res_unauthorized = unlock(&app, voter_cookie).await;
    assert_eq!(unlock_res_unauthorized.status(), StatusCode::UNAUTHORIZED);
    let unlock_res_authorized = unlock(&app, host_cookie).await;
    assert_eq!(unlock_res_authorized.status(), StatusCode::OK);
}
