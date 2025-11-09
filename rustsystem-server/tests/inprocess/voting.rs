//! - [x] Test one user (host)
//! - [x] Test one user (non-host)
//! - [x] Test multiple registrations
//! - [x] Test multiple re-registrations (after voting)
//! - [x] Test multiple votes
//! - [x] Test too many choices
//! - [x] Test vote invalid proof/token/signature
//! - [x] Test vote without vote active
//! - [x] Test vote during tally
//! - [ ] Test multiple users
//! - [ ] Test big tally score

use axum::http::StatusCode;
use rustsystem_proof::RegistrationSuccessResponse;
use rustsystem_server::{api::create_meeting::CreateMeetingResponse, vote_auth::Tally};
use uuid::Uuid;
use zkryptium::{
    keys::pair::KeyPair,
    schemes::{algorithms::BbsBls12381Sha256, generics::BlindSignature},
};

use crate::{
    common::MockApp,
    inprocess::{
        add_voter, clone_response, create_meeting, extract_cookie, parse_response_body, register,
        start_vote, tally, vote, voter_id, voter_login,
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

#[tokio::test]
async fn test_reg_not_active() {
    let app = MockApp::new_inprocess();
    let (creation_res1, creation_res2) = clone_response(create_meeting(&app).await).await;
    let cookie = extract_cookie(&creation_res1).1;

    let CreateMeetingResponse { uuuid, muuid } = parse_response_body(creation_res2).await;

    let (_token, _proof, reg_res) = register(&app, cookie, uuuid, muuid).await;
    assert_eq!(reg_res.status(), StatusCode::GONE);
}

#[tokio::test]
async fn test_reg_when_tally() {
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

    tally(&app, cookie).await;
    let (_token, _proof, reg_res) = register(&app, cookie, uuuid, muuid).await;
    assert_eq!(reg_res.status(), StatusCode::GONE);
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

#[tokio::test]
async fn test_invalid_proof() {
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

    let (token, _proof, reg_res) = register(&app, cookie, uuuid, muuid).await;
    let reg = parse_response_body::<RegistrationSuccessResponse>(reg_res).await;
    let vote_res = vote(&app, cookie, Some(vec![0, 1]), reg, token, vec![0u8; 32]).await;
    assert_eq!(vote_res.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn test_invalid_token() {
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

    let (_token, proof, reg_res) = register(&app, cookie, uuuid, muuid).await;
    let reg = parse_response_body::<RegistrationSuccessResponse>(reg_res).await;
    let vote_res = vote(
        &app,
        cookie,
        Some(vec![0, 1]),
        reg,
        vec![0u8; 32],
        proof.to_bytes().to_vec(),
    )
    .await;
    assert_eq!(vote_res.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn test_invalid_signature() {
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
    let mut reg = parse_response_body::<RegistrationSuccessResponse>(reg_res).await;

    // Modify signature so that it's no longer valid
    let key_pair = KeyPair::<BbsBls12381Sha256>::random().unwrap();
    reg.set_signature(
        BlindSignature::<BbsBls12381Sha256>::blind_sign(
            key_pair.private_key(),
            key_pair.public_key(),
            None,
            None,
            None,
        )
        .unwrap(),
    );

    let vote_res = vote(
        &app,
        cookie,
        Some(vec![0, 1]),
        reg,
        token,
        proof.to_bytes().to_vec(),
    )
    .await;
    assert_eq!(vote_res.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn test_many_users() {
    let app = MockApp::new_inprocess();
    let (creation_res1, creation_res2) = clone_response(create_meeting(&app).await).await;
    let host_cookie = extract_cookie(&creation_res1).1;

    let CreateMeetingResponse { uuuid, muuid } = parse_response_body(creation_res2).await;

    let mut users = vec![(uuuid, host_cookie.clone())];

    for i in 0..10 {
        let voter_name = format!("Voter{i}");
        let add_res = add_voter(&app, host_cookie, voter_name.clone(), false).await;
        let login_res = voter_login(&app, add_res).await;
        let voter_cookie = extract_cookie(&login_res).1;
        let id_res = voter_id(&app, host_cookie, voter_name).await;
        let id = parse_response_body::<Uuid>(id_res).await;
        users.push((id, voter_cookie.clone()));
    }

    start_vote(
        &app,
        host_cookie,
        vec!["Candidate1".into(), "Candidate2".into()],
        2,
    )
    .await;

    for (user_uuuid, cookie) in users {
        let (token, proof, reg_res) = register(&app, &cookie, user_uuuid, muuid).await;
        assert_eq!(reg_res.status(), StatusCode::CREATED);

        let reg = parse_response_body::<RegistrationSuccessResponse>(reg_res).await;
        let vote_res = vote(&app, &cookie, None, reg, token, proof.to_bytes().to_vec()).await;
        assert_eq!(vote_res.status(), StatusCode::OK);
    }
}

#[tokio::test]
async fn test_large_tally() {
    let app = MockApp::new_inprocess();
    let (creation_res1, creation_res2) = clone_response(create_meeting(&app).await).await;
    let host_cookie = extract_cookie(&creation_res1).1;

    let CreateMeetingResponse { uuuid, muuid } = parse_response_body(creation_res2).await;

    let mut users = vec![(uuuid, host_cookie.clone())];

    for i in 0..10 {
        let voter_name = format!("Voter{i}");
        let add_res = add_voter(&app, host_cookie, voter_name.clone(), false).await;
        let login_res = voter_login(&app, add_res).await;
        let voter_cookie = extract_cookie(&login_res).1;
        let id_res = voter_id(&app, host_cookie, voter_name).await;
        let id = parse_response_body::<Uuid>(id_res).await;
        users.push((id, voter_cookie.clone()));
    }

    start_vote(
        &app,
        host_cookie,
        vec![
            "Candidate1".into(),
            "Candidate2".into(),
            "Candidate3".into(),
            "Candidate4".into(),
            "Candidate5".into(),
            "Candidate6".into(),
        ],
        3,
    )
    .await;

    for (i, (user_uuuid, cookie)) in users.iter().enumerate() {
        let (token, proof, reg_res) = register(&app, &cookie, *user_uuuid, muuid).await;
        assert_eq!(reg_res.status(), StatusCode::CREATED);

        // non-trivial voting pattern
        let choices = if i % 4 == 0 {
            Some(vec![i % 6, (i + 1) % 6, (i + 2) % 6])
        } else if i % 3 == 0 {
            Some(vec![i % 6, (i + 1) % 6])
        } else if i % 2 == 0 {
            Some(vec![i % 6])
        } else {
            None
        };

        let reg = parse_response_body::<RegistrationSuccessResponse>(reg_res).await;
        let vote_res = vote(
            &app,
            &cookie,
            choices,
            reg,
            token,
            proof.to_bytes().to_vec(),
        )
        .await;
        assert_eq!(vote_res.status(), StatusCode::OK);
    }

    let tally_res = tally(&app, host_cookie).await;
    let tally_score = parse_response_body::<Tally>(tally_res).await;

    assert_eq!(*tally_score.score.get("Candidate1").unwrap(), 3); // {0, 4, 6}
    assert_eq!(*tally_score.score.get("Candidate2").unwrap(), 2); // {0, 6}
    assert_eq!(*tally_score.score.get("Candidate3").unwrap(), 3); // {0, 2, 8}
    assert_eq!(*tally_score.score.get("Candidate4").unwrap(), 3); // {3, 8, 9}
    assert_eq!(*tally_score.score.get("Candidate5").unwrap(), 5); // {3, 4, 8, 9, 10}
    assert_eq!(*tally_score.score.get("Candidate6").unwrap(), 1); // {4}
    assert_eq!(tally_score.blank, 3); // {1, 5, 7}
}
