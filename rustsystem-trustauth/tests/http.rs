//! Trustauth over HTTP: the full login → status → sign flow, the rules it must enforce, and
//! the request limits as a cross-origin browser sees them.

use std::net::SocketAddr;

use axum::{
    Router,
    body::Body,
    extract::ConnectInfo,
    http::{HeaderValue, Method, Request, Response, StatusCode, header},
};
use http_body_util::BodyExt;
use serde_json::{Value, json};
use tower::util::ServiceExt;
use uuid::Uuid;

use rustsystem_core::{
    blind::{RoundPublicKey, SIGNATURE_LEN, client},
    internal::{RoundStatus, StartRoundResponse},
    limits::{ClientIp, GENERAL, rate_limit_forced},
    secret::{b64_decode, b64_encode, new_token},
};
use rustsystem_trustauth::{AppState, internal_router, public_router};

const ORIGIN: &str = "http://localhost:1443";

fn routers(rate_limited: bool) -> (Router, Router) {
    let app = AppState::new(false);
    let limit = rate_limited.then(|| rate_limit_forced(GENERAL, ClientIp::default()));
    (
        public_router(app.clone(), vec![HeaderValue::from_static(ORIGIN)], limit),
        internal_router(app),
    )
}

fn request(method: Method, uri: &str, body: Option<Value>, cookie: Option<&str>) -> Request<Body> {
    let mut b = Request::builder()
        .method(method)
        .uri(uri)
        .header(header::ORIGIN, ORIGIN);
    if let Some(c) = cookie {
        b = b.header(header::COOKIE, c);
    }
    let body = match body {
        Some(v) => {
            b = b.header(header::CONTENT_TYPE, "application/json");
            Body::from(v.to_string())
        }
        None => Body::empty(),
    };
    let mut req = b.body(body).unwrap();
    req.extensions_mut()
        .insert(ConnectInfo(SocketAddr::from(([127, 0, 0, 1], 40000))));
    req
}

async fn send(router: &Router, req: Request<Body>) -> (StatusCode, Value, Response<Body>) {
    let res = router.clone().oneshot(req).await.unwrap();
    let status = res.status();
    let (parts, body) = res.into_parts();
    let bytes = body.collect().await.unwrap().to_bytes();
    let json = serde_json::from_slice(&bytes).unwrap_or(Value::Null);
    (status, json, Response::from_parts(parts, Body::empty()))
}

fn session_cookie(res: &Response<Body>) -> String {
    let set = res.headers()[header::SET_COOKIE].to_str().unwrap();
    assert!(set.contains("HttpOnly") && set.contains("SameSite=Strict"), "{set}");
    set.split(';').next().unwrap().to_owned()
}

struct Voter {
    meeting: Uuid,
    voter: Uuid,
    cookie: String,
}

/// The server issues a ticket; the browser exchanges it for a session.
async fn log_in(public: &Router, internal: &Router, meeting: Uuid) -> Voter {
    let voter = Uuid::new_v4();
    let ticket = new_token();
    let (status, ..) = send(
        internal,
        request(
            Method::POST,
            "/internal/tickets",
            Some(json!({ "ticket": ticket, "meeting": meeting, "voter": voter })),
            None,
        ),
    )
    .await;
    assert_eq!(status, StatusCode::NO_CONTENT);

    let (status, _, res) = send(
        public,
        request(Method::POST, "/api/login", Some(json!({ "ticket": ticket })), None),
    )
    .await;
    assert_eq!(status, StatusCode::NO_CONTENT);
    Voter {
        meeting,
        voter,
        cookie: session_cookie(&res),
    }
}

async fn start_round(internal: &Router, meeting: Uuid, eligible: &[Uuid]) -> (Uuid, RoundPublicKey) {
    let round = Uuid::new_v4();
    let (status, body, _) = send(
        internal,
        request(
            Method::POST,
            "/internal/rounds",
            Some(json!({ "meeting": meeting, "round": round, "eligible": eligible })),
            None,
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    let resp: StartRoundResponse = serde_json::from_value(body).unwrap();
    let pk = RoundPublicKey::from_der(&b64_decode(&resp.public_key).unwrap()).unwrap();
    (round, pk)
}

async fn sign(public: &Router, v: &Voter, round: Uuid, blinded: &[u8]) -> (StatusCode, Value) {
    let (status, body, _) = send(
        public,
        request(
            Method::POST,
            "/api/sign",
            Some(json!({ "round": round, "blinded": b64_encode(blinded) })),
            Some(&v.cookie),
        ),
    )
    .await;
    (status, body)
}

#[tokio::test]
async fn full_flow_produces_a_valid_unlinkable_signature() {
    let (public, internal) = routers(false);
    let meeting = Uuid::new_v4();
    let v = log_in(&public, &internal, meeting).await;
    let (round, pk) = start_round(&internal, meeting, &[v.voter]).await;

    let (_, body, _) = send(&public, request(Method::GET, "/api/status", None, Some(&v.cookie))).await;
    assert_eq!(body, json!({ "round": round, "signed": false }));

    let ballot = format!(r#"{{"v":1,"round":"{round}","choice":null,"nonce":"n"}}"#);
    let blinded = client::blind(&pk, ballot.as_bytes()).unwrap();
    let (code, body) = sign(&public, &v, round, &blinded.blinded).await;
    assert_eq!(code, StatusCode::OK, "{body}");
    let blind_sig = b64_decode(body["blind_sig"].as_str().unwrap()).unwrap();

    let (prepared, sig) = client::finalize(&pk, &blinded, &blind_sig).unwrap();
    assert_eq!(pk.verify(&prepared, &sig).unwrap(), ballot.as_bytes());

    let (_, body, _) = send(&public, request(Method::GET, "/api/status", None, Some(&v.cookie))).await;
    assert_eq!(body, json!({ "round": round, "signed": true }));

    let (_, rs, _) = send(&internal, request(Method::GET, &format!("/internal/rounds/{meeting}"), None, None)).await;
    let rs: RoundStatus = serde_json::from_value(rs).unwrap();
    assert_eq!(rs, RoundStatus { round: Some(round), signed: 1 });
}

#[tokio::test]
async fn second_signature_is_refused() {
    let (public, internal) = routers(false);
    let meeting = Uuid::new_v4();
    let v = log_in(&public, &internal, meeting).await;
    let (round, pk) = start_round(&internal, meeting, &[v.voter]).await;

    let first = client::blind(&pk, b"a").unwrap();
    assert_eq!(sign(&public, &v, round, &first.blinded).await.0, StatusCode::OK);
    let second = client::blind(&pk, b"b").unwrap();
    let (code, body) = sign(&public, &v, round, &second.blinded).await;
    assert_eq!(code, StatusCode::CONFLICT);
    assert_eq!(body["code"], "AlreadySigned");
}

#[tokio::test]
async fn ineligible_and_wrong_round_are_refused() {
    let (public, internal) = routers(false);
    let meeting = Uuid::new_v4();
    let v = log_in(&public, &internal, meeting).await;
    let (round, pk) = start_round(&internal, meeting, &[Uuid::new_v4()]).await;
    let blinded = client::blind(&pk, b"x").unwrap();

    let (code, body) = sign(&public, &v, round, &blinded.blinded).await;
    assert_eq!((code, body["code"].clone()), (StatusCode::FORBIDDEN, json!("NotEligible")));

    let (code, body) = sign(&public, &v, Uuid::new_v4(), &blinded.blinded).await;
    assert_eq!((code, body["code"].clone()), (StatusCode::CONFLICT, json!("WrongRound")));
}

#[tokio::test]
async fn malformed_blinded_message_does_not_use_up_the_signature() {
    let (public, internal) = routers(false);
    let meeting = Uuid::new_v4();
    let v = log_in(&public, &internal, meeting).await;
    let (round, pk) = start_round(&internal, meeting, &[v.voter]).await;

    let (code, body) = sign(&public, &v, round, &[1u8; 10]).await;
    assert_eq!((code, body["code"].clone()), (StatusCode::BAD_REQUEST, json!("MalformedBallot")));
    // All-0xFF is the right length but larger than the modulus: rejected by the signer.
    let (code, _) = sign(&public, &v, round, &[0xFFu8; SIGNATURE_LEN]).await;
    assert_eq!(code, StatusCode::BAD_REQUEST);

    let good = client::blind(&pk, b"x").unwrap();
    assert_eq!(sign(&public, &v, round, &good.blinded).await.0, StatusCode::OK);
}

#[tokio::test]
async fn sessions_are_required_and_revocable() {
    let (public, internal) = routers(false);
    let meeting = Uuid::new_v4();

    let (code, body, _) = send(&public, request(Method::GET, "/api/status", None, None)).await;
    assert_eq!((code, body["code"].clone()), (StatusCode::UNAUTHORIZED, json!("NotLoggedIn")));
    let (code, body, _) = send(&public, request(Method::GET, "/api/status", None, Some("ta_session=bogus"))).await;
    assert_eq!((code, body["code"].clone()), (StatusCode::UNAUTHORIZED, json!("SessionExpired")));

    let v = log_in(&public, &internal, meeting).await;
    send(&internal, request(Method::DELETE, &format!("/internal/voters/{}/{}", v.meeting, v.voter), None, None)).await;
    let (code, ..) = send(&public, request(Method::GET, "/api/status", None, Some(&v.cookie))).await;
    assert_eq!(code, StatusCode::UNAUTHORIZED);

    let w = log_in(&public, &internal, meeting).await;
    send(&internal, request(Method::DELETE, &format!("/internal/meetings/{meeting}"), None, None)).await;
    let (code, ..) = send(&public, request(Method::GET, "/api/status", None, Some(&w.cookie))).await;
    assert_eq!(code, StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn ticket_works_once() {
    let (public, internal) = routers(false);
    let ticket = new_token();
    send(&internal, request(Method::POST, "/internal/tickets",
        Some(json!({ "ticket": ticket, "meeting": Uuid::new_v4(), "voter": Uuid::new_v4() })), None)).await;
    let login = || request(Method::POST, "/api/login", Some(json!({ "ticket": ticket })), None);
    assert_eq!(send(&public, login()).await.0, StatusCode::NO_CONTENT);
    let (code, body, _) = send(&public, login()).await;
    assert_eq!((code, body["code"].clone()), (StatusCode::UNAUTHORIZED, json!("TicketInvalid")));
}

#[tokio::test]
async fn bad_json_gets_a_json_error() {
    let (public, _) = routers(false);
    let mut req = request(Method::POST, "/api/login", None, None);
    *req.body_mut() = Body::from("{not json");
    req.headers_mut().insert(header::CONTENT_TYPE, HeaderValue::from_static("application/json"));
    let (code, body, _) = send(&public, req).await;
    assert_eq!((code, body["code"].clone()), (StatusCode::BAD_REQUEST, json!("InvalidInput")));
}

// ── Limits, as a cross-origin browser sees them ──────────────────────────────

fn allows_origin(res: &Response<Body>) -> bool {
    res.headers().get(header::ACCESS_CONTROL_ALLOW_ORIGIN) == Some(&HeaderValue::from_static(ORIGIN))
}

#[tokio::test]
async fn oversized_body_is_413_and_readable_cross_origin() {
    let (public, _) = routers(true);
    let mut req = request(Method::POST, "/api/login", None, None);
    *req.body_mut() = Body::from(vec![b'x'; 128 * 1024]);
    req.headers_mut().insert(header::CONTENT_TYPE, HeaderValue::from_static("application/json"));
    let (code, body, res) = send(&public, req).await;
    assert_eq!((code, body["code"].clone()), (StatusCode::PAYLOAD_TOO_LARGE, json!("BodyTooLarge")));
    assert!(allows_origin(&res), "without CORS headers the voter only sees 'Failed to fetch'");
}

#[tokio::test]
async fn rate_limit_is_429_and_readable_cross_origin() {
    let (public, _) = routers(true);
    let login = || request(Method::POST, "/api/login", Some(json!({ "ticket": "x" })), None);
    for _ in 0..GENERAL.burst + 50 {
        let (code, body, res) = send(&public, login()).await;
        if code == StatusCode::TOO_MANY_REQUESTS {
            assert_eq!(body["code"], "RateLimited");
            assert!(allows_origin(&res));
            return;
        }
    }
    panic!("hammering one IP past the burst must produce a 429");
}

#[tokio::test]
async fn preflight_is_allowed_and_never_throttled() {
    let (public, _) = routers(true);
    for _ in 0..GENERAL.burst + 50 {
        let mut req = request(Method::OPTIONS, "/api/sign", None, None);
        req.headers_mut().insert(header::ACCESS_CONTROL_REQUEST_METHOD, HeaderValue::from_static("POST"));
        req.headers_mut().insert(header::ACCESS_CONTROL_REQUEST_HEADERS, HeaderValue::from_static("content-type"));
        let (code, _, res) = send(&public, req).await;
        assert_ne!(code, StatusCode::TOO_MANY_REQUESTS);
        assert!(allows_origin(&res));
    }
}

#[tokio::test]
async fn other_origins_are_not_allowed() {
    let (public, _) = routers(false);
    let mut req = request(Method::GET, "/api/status", None, None);
    req.headers_mut().insert(header::ORIGIN, HeaderValue::from_static("https://evil.example"));
    let (_, _, res) = send(&public, req).await;
    assert!(res.headers().get(header::ACCESS_CONTROL_ALLOW_ORIGIN).is_none());
}
