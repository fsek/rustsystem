use axum::{
    body::Body,
    http::{HeaderName, HeaderValue, Method},
    response::Response,
};
use rustsystem_proof::BallotMetaData;

use crate::{
    common::{MockApp, json_request},
    inprocess::qr_reader::{extract_url_args, read_qr},
};

use http_body_util::BodyExt;
use rustsystem_server::api::{
    create_meeting::CreateMeetingRequest,
    host::{new_voter::NewVoterRequestBody, state::StartVoteRequest},
    login::LoginRequest,
};

mod creation;
mod permissions;
mod qr_reader;
mod sequence;

async fn create_meeting(app: &MockApp) -> Response {
    let title = String::from("Test Meeting");
    let host_name = String::from("Creator");

    app.oneshot(json_request(
        Method::POST,
        "/api/create-meeting",
        serde_json::to_value(CreateMeetingRequest { title, host_name }).unwrap(),
        None,
    ))
    .await
}

fn extract_cookie(res: &Response) -> (&HeaderName, &HeaderValue) {
    res.headers()
        .iter()
        .find(|(name, _value)| name.as_str() == "set-cookie")
        .unwrap()
}

async fn add_voter(
    app: &MockApp,
    cookie: &HeaderValue,
    name: impl Into<String>,
    is_host: bool,
) -> Response {
    app.oneshot(json_request(
        Method::POST,
        "/api/host/new-voter",
        serde_json::to_value(NewVoterRequestBody {
            voter_name: name.into(),
            is_host,
        })
        .unwrap(),
        Some(cookie.clone()),
    ))
    .await
}

async fn voter_login(app: &MockApp, res: Response) -> Response {
    let collected = res.into_body().collect().await.unwrap();
    let bytes = collected.to_bytes();
    let body_str = String::from_utf8(bytes.to_vec()).unwrap();

    let url = read_qr(&body_str).unwrap();

    let (uuuid, muuid) = extract_url_args(&url).unwrap();

    app.oneshot(json_request(
        Method::POST,
        "/api/login",
        serde_json::to_value(LoginRequest {
            uuuid: uuuid,
            muuid: muuid,
            admin_cred: None,
        })
        .unwrap(),
        None,
    ))
    .await
}

async fn start_vote(app: &MockApp, cookie: &HeaderValue) -> Response {
    app.oneshot(json_request(
        Method::POST,
        "/api/host/start-vote",
        serde_json::to_value(StartVoteRequest {
            name: String::from("Some Vote Round"),
            shuffle: false,
            metadata: BallotMetaData::new(
                vec![
                    "Candidate1".into(),
                    "Candidate2".into(),
                    "Candidate3".into(),
                ],
                1,
                3,
            ),
        })
        .unwrap(),
        Some(cookie.clone()),
    ))
    .await
}

async fn tally(app: &MockApp, cookie: &HeaderValue) -> Response {
    app.oneshot(json_request(
        Method::POST,
        "/api/host/tally",
        serde_json::to_value(()).unwrap(),
        Some(cookie.clone()),
    ))
    .await
}

async fn end_vote_round(app: &MockApp, cookie: &HeaderValue) -> Response {
    app.oneshot(json_request(
        Method::DELETE,
        "/api/host/end-vote-round",
        serde_json::to_value(()).unwrap(),
        Some(cookie.clone()),
    ))
    .await
}

async fn lock(app: &MockApp, cookie: &HeaderValue) -> Response {
    app.oneshot(json_request(
        Method::POST,
        "/api/host/lock",
        serde_json::to_value(()).unwrap(),
        Some(cookie.clone()),
    ))
    .await
}

async fn unlock(app: &MockApp, cookie: &HeaderValue) -> Response {
    app.oneshot(json_request(
        Method::POST,
        "/api/host/unlock",
        serde_json::to_value(()).unwrap(),
        Some(cookie.clone()),
    ))
    .await
}

// This is a really awkward and somewhat inefficient way of getting clones of the response.
// In production, this would be terrible, but for testing it's fine.
async fn clone_response(res: Response) -> (Response, Response) {
    let (parts, body) = res.into_parts();
    let bytes = body.collect().await.unwrap().to_bytes();

    let res1 = Response::from_parts(parts.clone(), Body::from(bytes.clone()));
    let res2 = Response::from_parts(parts, Body::from(bytes.clone()));

    return (res1, res2);
}
