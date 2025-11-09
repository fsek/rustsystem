// Test one user (host)
// Test one user (non-host)
// Test multiple users
// Test multiple registrations
// Test multiple votes
// Test vote without reg
// Test vote without vote active
// Test vote during tally
// Test vote without cookie
// Test vote invalid signature/proof
// Test big tally score

use axum::http::StatusCode;
use rustsystem_proof::RegistrationSuccessResponse;
use rustsystem_server::api::create_meeting::CreateMeetingResponse;

use crate::{
    common::MockApp,
    inprocess::{
        clone_response, create_meeting, extract_cookie, parse_response_body, register, start_vote,
        vote,
    },
};

#[tokio::test]
async fn test_one_vote_host() {
    let app = MockApp::new_inprocess();
    let (creation_res1, creation_res2) = clone_response(create_meeting(&app).await).await;
    let cookie = extract_cookie(&creation_res1).1;

    let CreateMeetingResponse { uuuid, muuid } = parse_response_body(creation_res2).await;

    start_vote(
        &app,
        cookie,
        vec!["Candidate1".into(), "Candidate2".into()],
        2,
    )
    .await;

    let (token, proof, reg_res) = register(&app, cookie, uuuid, muuid).await;
    assert_eq!(reg_res.status(), StatusCode::CREATED);

    let reg = parse_response_body::<RegistrationSuccessResponse>(reg_res).await;
    let vote_res = vote(&app, cookie, None, reg, token, proof.to_bytes().to_vec()).await;
    assert_eq!(vote_res.status(), StatusCode::OK);
}
