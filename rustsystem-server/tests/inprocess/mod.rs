use axum::{
    body::Body,
    http::{HeaderName, HeaderValue, Method},
    response::Response,
};
use rustsystem_proof::{
    Ballot, BallotMetaData, BallotValidation, Candidates, Choice, ProofContext, Provider,
    RegistrationSuccessResponse, Sha256Provider, Sha256RegistrationInfo,
};
use serde::{Deserialize, de::DeserializeOwned};
use uuid::Uuid;
use zkryptium::bbsplus::commitment::BlindFactor;

use crate::{
    common::{MockApp, json_request},
    inprocess::qr_reader::{extract_url_args, read_qr},
};

use http_body_util::BodyExt;
use rustsystem_server::api::{
    create_meeting::CreateMeetingRequest,
    host::{
        new_voter::NewVoterRequestBody,
        start_vote::StartVoteRequest,
        remove_voter::RemoveVoterRequest,
        reset_login::ResetLoginRequest,
        voter_id::VoterIdRequest,
    },
    login::LoginRequest,
};

mod creation;
mod management;
mod permissions;
mod qr_reader;
mod sequence;
mod voting;

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

async fn register(
    app: &MockApp,
    cookie: &HeaderValue,
    voter_id: Uuid,
    meeting_id: Uuid,
) -> (Vec<u8>, BlindFactor, Response) {
    let (context, token, commitment, proof) = Sha256Provider::generate_token(
        voter_id.into_bytes().to_vec(),
        meeting_id.into_bytes().to_vec(),
    )
    .unwrap();
    let reginfo = Sha256Provider::new_reg_info(context, commitment);

    (
        token,
        proof,
        app.oneshot(json_request(
            Method::POST,
            "/api/voter/register",
            serde_json::to_value(reginfo).unwrap(),
            Some(cookie.clone()),
        ))
        .await,
    )
}

async fn vote(
    app: &MockApp,
    cookie: &HeaderValue,
    choice: Option<Choice>,
    regres: RegistrationSuccessResponse,
    token: Vec<u8>,
    proof: Vec<u8>,
) -> Response {
    let validation = BallotValidation::new(proof, token, regres.get_signature().clone());
    let ballot = Ballot::new(regres.get_metadata().clone(), choice, validation);
    app.oneshot(json_request(
        Method::POST,
        "/api/voter/submit",
        serde_json::to_value(ballot).unwrap(),
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
