//! - [x] Test one user (host)
//! - [x] Test one user (non-host)
//! - [x] Test multiple registrations
//! - [x] Test multiple re-registrations (after voting)
//! - [x] Test multiple votes
//! - [x] Test too many choices
//! - [ ] Test vote invalid signature/proof
//! - [ ] Test vote without cookie
//! - [ ] Test vote without vote active
//! - [ ] Test vote during tally
//! - [ ] Test multiple users
//! - [ ] Test big tally score

use axum::http::StatusCode;
use rustsystem_proof::RegistrationSuccessResponse;
use rustsystem_server::{api::create_meeting::CreateMeetingResponse, vote_auth::Tally};

use crate::{
    common::MockApp,
    inprocess::{
        add_voter, clone_response, create_meeting, extract_cookie, parse_response_body, register,
        start_vote, tally, vote, voter_login,
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

    let tally_res = tally(&app, cookie).await;
    let tally_score = parse_response_body::<Tally>(tally_res).await;

    assert_eq!(tally_score.blank, 1);
    for (_candidate, score) in tally_score.score.iter() {
        assert_eq!(*score, 0);
    }
}

#[tokio::test]
async fn test_one_vote_non_host() {
    let app = MockApp::new_inprocess();
    let (creation_res1, creation_res2) = clone_response(create_meeting(&app).await).await;
    let host_cookie = extract_cookie(&creation_res1).1;

    let CreateMeetingResponse { uuuid, muuid } = parse_response_body(creation_res2).await;

    let add_res = add_voter(&app, host_cookie, format!("Voter"), false).await;
    let login_res = voter_login(&app, add_res).await;
    let voter_cookie = extract_cookie(&login_res).1;

    start_vote(
        &app,
        host_cookie,
        vec!["Candidate1".into(), "Candidate2".into()],
        2,
    )
    .await;

    let (token, proof, reg_res) = register(&app, voter_cookie, uuuid, muuid).await;
    assert_eq!(reg_res.status(), StatusCode::CREATED);

    let reg = parse_response_body::<RegistrationSuccessResponse>(reg_res).await;
    let vote_res = vote(
        &app,
        voter_cookie,
        Some(vec![0, 1]),
        reg,
        token,
        proof.to_bytes().to_vec(),
    )
    .await;
    assert_eq!(vote_res.status(), StatusCode::OK);

    let tally_res = tally(&app, host_cookie).await;
    let tally_score = parse_response_body::<Tally>(tally_res).await;

    assert_eq!(tally_score.blank, 0);
    for (_candidate, score) in tally_score.score.iter() {
        assert_eq!(*score, 1);
    }
}

// Voters should not be allowed to register multiple times!
#[tokio::test]
async fn test_multiple_registration() {
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

    let (_token, _proof, _reg_res) = register(&app, cookie, uuuid, muuid).await;

    let (_token, _proof, reg_res) = register(&app, cookie, uuuid, muuid).await;
    assert_eq!(reg_res.status(), StatusCode::CONFLICT);
}

// Voters should not be allowed to re-register after having voted!
#[tokio::test]
async fn test_reregistration() {
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
    let reg = parse_response_body::<RegistrationSuccessResponse>(reg_res).await;
    vote(&app, cookie, None, reg, token, proof.to_bytes().to_vec()).await;

    // Registering here (After placing vote) should still fail!
    let (_token, _proof, reg_res) = register(&app, cookie, uuuid, muuid).await;
    assert_eq!(reg_res.status(), StatusCode::CONFLICT);
}

// Voters should not be allowed to vote again after having placed their vote!
#[tokio::test]
async fn test_multiple_votes() {
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
    let reg = parse_response_body::<RegistrationSuccessResponse>(reg_res).await;
    let _vote_res = vote(
        &app,
        cookie,
        None,
        reg.clone(),
        token.clone(),
        proof.to_bytes().to_vec(),
    )
    .await;

    let vote_res = vote(&app, cookie, None, reg, token, proof.to_bytes().to_vec()).await;
    assert_eq!(vote_res.status(), StatusCode::CONFLICT);
}

#[tokio::test]
async fn test_too_many_choices() {
    let app = MockApp::new_inprocess();
    let (creation_res1, creation_res2) = clone_response(create_meeting(&app).await).await;
    let cookie = extract_cookie(&creation_res1).1;

    let CreateMeetingResponse { uuuid, muuid } = parse_response_body(creation_res2).await;

    start_vote(
        &app,
        cookie,
        vec!["Candidate1".into(), "Candidate2".into()],
        1,
    )
    .await;

    let (token, proof, reg_res) = register(&app, cookie, uuuid, muuid).await;
    let reg = parse_response_body::<RegistrationSuccessResponse>(reg_res).await;
    let vote_res = vote(
        &app,
        cookie,
        Some(vec![0, 1]),
        reg,
        token,
        proof.to_bytes().to_vec(),
    )
    .await;
    assert_eq!(vote_res.status(), StatusCode::CONFLICT);
}
