use axum::http::StatusCode;
use uuid::Uuid;

use crate::{
    common::MockApp,
    inprocess::{
        add_voter, create_meeting, extract_cookie, parse_response_body, remove_voter, voter_id,
        voter_list, voter_login,
    },
};

#[tokio::test]
async fn simple_list() {
    let app = MockApp::new_inprocess();
    let creation_res = create_meeting(&app).await;
    let cookie = extract_cookie(&creation_res).1;

    let list_res = voter_list(&app, cookie).await;
    assert_eq!(list_res.status(), StatusCode::OK);
    let voters = parse_response_body::<Vec<(String, String)>>(list_res).await;
    assert_eq!(voters.len(), 1);
    assert_eq!(voters[0].0, "Creator");
}

#[tokio::test]
async fn test_get_voter_id() {
    let app = MockApp::new_inprocess();
    let creation_res = create_meeting(&app).await;
    let cookie = extract_cookie(&creation_res).1;

    let id_res = voter_id(&app, cookie, String::from("Creator")).await;
    assert_eq!(id_res.status(), StatusCode::OK);
    let _id = parse_response_body::<Uuid>(id_res).await; // Make sure it's parsable
}

#[tokio::test]
async fn test_remove_one() {
    let app = MockApp::new_inprocess();
    let creation_res = create_meeting(&app).await;
    let cookie = extract_cookie(&creation_res).1;

    let id_res = voter_id(&app, cookie, String::from("Creator")).await;
    let id = parse_response_body::<Uuid>(id_res).await;
    let remove_res = remove_voter(&app, cookie, id).await;

    assert_eq!(remove_res.status(), StatusCode::OK);

    // check list of users
    let list_res = voter_list(&app, cookie).await;
    let voters = parse_response_body::<Vec<(String, String)>>(list_res).await;
    assert_eq!(voters.len(), 0);
}

#[tokio::test]
async fn test_remove_non_existing() {
    let app = MockApp::new_inprocess();
    let creation_res = create_meeting(&app).await;
    let cookie = extract_cookie(&creation_res).1;

    // New random uuid will be different and this request should fail
    let remove_res = remove_voter(&app, cookie, Uuid::new_v4()).await;
    assert_eq!(remove_res.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn test_add_remove_one() {
    let app = MockApp::new_inprocess();
    let creation_res = create_meeting(&app).await;
    let cookie = extract_cookie(&creation_res).1;

    let add_res = add_voter(&app, cookie, format!("Voter"), false).await;
    voter_login(&app, add_res).await;

    // check list of users
    let list_res = voter_list(&app, cookie).await;
    let voters = parse_response_body::<Vec<(String, String)>>(list_res).await;
    assert_eq!(voters.len(), 2);

    let id_res = voter_id(&app, cookie, String::from("Voter")).await;
    let id = parse_response_body::<Uuid>(id_res).await;
    let remove_res = remove_voter(&app, cookie, id).await;

    assert_eq!(remove_res.status(), StatusCode::OK);

    // check list of users
    let list_res = voter_list(&app, cookie).await;
    let voters = parse_response_body::<Vec<(String, String)>>(list_res).await;
    assert_eq!(voters.len(), 1);
}

#[tokio::test]
async fn test_remove_pending() {
    let app = MockApp::new_inprocess();
    let creation_res = create_meeting(&app).await;
    let cookie = extract_cookie(&creation_res).1;

    add_voter(&app, cookie, format!("Voter"), false).await;

    // check list of users
    let list_res = voter_list(&app, cookie).await;
    let voters = parse_response_body::<Vec<(String, String)>>(list_res).await;
    assert_eq!(voters.len(), 2);

    let id_res = voter_id(&app, cookie, String::from("Voter")).await;
    let id = parse_response_body::<Uuid>(id_res).await;
    let remove_res = remove_voter(&app, cookie, id).await;

    assert_eq!(remove_res.status(), StatusCode::OK);

    // check list of users
    let list_res = voter_list(&app, cookie).await;
    let voters = parse_response_body::<Vec<(String, String)>>(list_res).await;
    assert_eq!(voters.len(), 1);
}

#[tokio::test]
async fn test_add_remove_many() {
    let app = MockApp::new_inprocess();
    let creation_res = create_meeting(&app).await;
    let cookie = extract_cookie(&creation_res).1;

    for i in 0..10 {
        let add_res = add_voter(&app, cookie, format!("Voter{i}"), false).await;
        voter_login(&app, add_res).await;
    }

    for i in 0..10 {
        let id_res = voter_id(&app, cookie, format!("Voter{i}")).await;
        let id = parse_response_body::<Uuid>(id_res).await;
        let remove_res = remove_voter(&app, cookie, id).await;
        assert_eq!(remove_res.status(), StatusCode::OK);

        // check list of users
        let list_res = voter_list(&app, cookie).await;
        let voters = parse_response_body::<Vec<(String, String)>>(list_res).await;
        assert_eq!(voters.len(), 10 - i);
    }
}
