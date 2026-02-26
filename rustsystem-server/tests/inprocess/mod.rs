use axum::{
    body::Body,
    http::{HeaderName, HeaderValue, Method},
    response::Response,
};
use serde::de::DeserializeOwned;
use url::Url;
use uuid::Uuid;

use crate::common::{MockApp, json_request};

use http_body_util::BodyExt;
use rustsystem_server::{
    api::{
        create_meeting::CreateMeetingRequest,
        host::{
            new_voter::{NewVoterRequestBody, QrCodeResponse},
            remove_voter::RemoveVoterRequest,
            reset_login::ResetLoginRequest,
            start_vote::StartVoteRequest,
            voter_id::VoterIdRequest,
        },
        login::LoginRequest,
    },
    proof::{BallotMetaData, Candidates},
};

mod auth;
mod common_endpoints;
mod concurrency;
mod creation;
mod lifecycle;
mod management;
mod negative;
mod state_machine;

async fn create_meeting(app: &MockApp) -> Response {
    let title = String::from("Test Meeting");
    let host_name = String::from("Creator");

    app.oneshot(json_request(
        Method::POST,
        "/api/create-meeting",
        serde_json::to_value(CreateMeetingRequest {
            title,
            host_name,
            pub_key: String::new(),
        })
        .unwrap(),
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
    let bytes = res.into_body().collect().await.unwrap().to_bytes();
    let qr_res: QrCodeResponse = serde_json::from_slice(&bytes).unwrap();

    let url = Url::parse(&qr_res.invite_link).unwrap();
    let params: std::collections::HashMap<_, _> = url.query_pairs().into_owned().collect();
    let uuuid = params["uuuid"].to_string();
    let muuid = params["muuid"].to_string();

    app.oneshot(json_request(
        Method::POST,
        "/api/login",
        serde_json::to_value(LoginRequest {
            uuuid,
            muuid,
            admin_cred: None,
        })
        .unwrap(),
        None,
    ))
    .await
}

async fn start_vote(
    app: &MockApp,
    cookie: &HeaderValue,
    candidates: Candidates,
    max_votes: usize,
) -> Response {
    app.oneshot(json_request(
        Method::POST,
        "/api/host/start-vote",
        serde_json::to_value(StartVoteRequest {
            name: String::from("Some Vote Round"),
            shuffle: false,
            metadata: BallotMetaData::new(candidates, 1, max_votes),
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

async fn voter_list(app: &MockApp, cookie: &HeaderValue) -> Response {
    app.oneshot(json_request(
        Method::GET,
        "/api/host/voter-list",
        serde_json::to_value(()).unwrap(),
        Some(cookie.clone()),
    ))
    .await
}

async fn voter_id(app: &MockApp, cookie: &HeaderValue, name: String) -> Response {
    app.oneshot(json_request(
        Method::GET,
        "/api/host/voter-id",
        serde_json::to_value(VoterIdRequest { name }).unwrap(),
        Some(cookie.clone()),
    ))
    .await
}

async fn remove_voter(app: &MockApp, cookie: &HeaderValue, uuuid: Uuid) -> Response {
    app.oneshot(json_request(
        Method::DELETE,
        "/api/host/remove-voter",
        serde_json::to_value(RemoveVoterRequest { voter_uuuid: uuuid }).unwrap(),
        Some(cookie.clone()),
    ))
    .await
}

async fn reset_login(app: &MockApp, cookie: &HeaderValue, uuuid: Uuid) -> Response {
    app.oneshot(json_request(
        Method::POST,
        "/api/host/reset-login",
        serde_json::to_value(ResetLoginRequest { user_uuuid: uuuid }).unwrap(),
        Some(cookie.clone()),
    ))
    .await
}

async fn close_meeting(app: &MockApp, cookie: &HeaderValue) -> Response {
    app.oneshot(json_request(
        Method::DELETE,
        "/api/host/close-meeting",
        serde_json::to_value(()).unwrap(),
        Some(cookie.clone()),
    ))
    .await
}

async fn remove_all(app: &MockApp, cookie: &HeaderValue) -> Response {
    app.oneshot(json_request(
        Method::DELETE,
        "/api/host/remove-all",
        serde_json::to_value(()).unwrap(),
        Some(cookie.clone()),
    ))
    .await
}

async fn get_tally(app: &MockApp, cookie: &HeaderValue) -> Response {
    app.oneshot(json_request(
        Method::GET,
        "/api/host/get-tally",
        serde_json::to_value(()).unwrap(),
        Some(cookie.clone()),
    ))
    .await
}

async fn get_all_tally(app: &MockApp, cookie: &HeaderValue) -> Response {
    app.oneshot(json_request(
        Method::GET,
        "/api/host/get-all-tally",
        serde_json::to_value(()).unwrap(),
        Some(cookie.clone()),
    ))
    .await
}

async fn vote_progress(app: &MockApp, cookie: &HeaderValue) -> Response {
    app.oneshot(json_request(
        Method::GET,
        "/api/common/vote-progress",
        serde_json::to_value(()).unwrap(),
        Some(cookie.clone()),
    ))
    .await
}

async fn meeting_specs(app: &MockApp, cookie: &HeaderValue) -> Response {
    app.oneshot(json_request(
        Method::GET,
        "/api/common/meeting-specs",
        serde_json::to_value(()).unwrap(),
        Some(cookie.clone()),
    ))
    .await
}

// ---------- helper functions ----------

async fn parse_response_body<T: DeserializeOwned>(res: Response) -> T {
    let body = res.into_body();
    let bytes = body.collect().await.unwrap().to_bytes();
    return serde_json::from_slice(&bytes).unwrap();
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
