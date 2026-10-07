//! End-to-end: the real server and trustauth, driven the way browsers drive them.
//! Each test names the rule from `docs/PROTOCOL.md` it checks.

mod common;

use std::time::Duration;

use base64::{Engine, engine::general_purpose::STANDARD};
use chacha20poly1305::{
    ChaCha20Poly1305, Key, Nonce,
    aead::{Aead, KeyInit, Payload},
};
use hkdf::Hkdf;
use reqwest::StatusCode;
use serde_json::{Value, json};
use sha2::Sha256;
use x25519_dalek::x25519;

use common::{Browser, TestEnv, error_code};

/// A meeting with a host and `n` logged-in voters.
async fn meeting(env: &TestEnv, n: usize) -> (Browser, Vec<Browser>, common::TallyKey) {
    let mut host = env.browser();
    let key = host.create_meeting("Vårmöte", "Host").await;
    let mut voters = Vec::new();
    for i in 0..n {
        let invite = host.invite(&format!("Voter {i}"), false).await;
        let mut b = env.browser();
        b.join(&invite).await;
        voters.push(b);
    }
    (host, voters, key)
}

fn decrypt_tally(file: &[u8], scalar: [u8; 32]) -> Value {
    let header = &file[..32];
    let eph: [u8; 32] = file[32..64].try_into().unwrap();
    let shared = x25519(scalar, eph);
    let mut okm = [0u8; 44];
    Hkdf::<Sha256>::new(Some(&eph), &shared).expand(b"rustsystem-tally-v2", &mut okm).unwrap();
    #[allow(deprecated)]
    let plain = ChaCha20Poly1305::new(Key::from_slice(&okm[..32]))
        .decrypt(Nonce::from_slice(&file[64..76]), Payload { msg: &file[76..], aad: header })
        .expect("tally decrypts with the password-derived key");
    serde_json::from_slice(&plain).unwrap()
}

// ── The whole meeting ────────────────────────────────────────────────────────

#[tokio::test]
async fn full_meeting() {
    let env = TestEnv::start().await;
    let (host, voters, key) = meeting(&env, 3).await;

    let (_, m) = host.get("/api/meeting").await;
    assert_eq!((m["title"].clone(), m["participants"].clone(), m["phase"].clone()), (json!("Vårmöte"), json!(4), json!("idle")));

    host.start_round("Chair", &["Anna", "Bo", "Cecilia"], 2).await;
    host.vote(json!([0, 2])).await;
    voters[0].vote(json!([2])).await;
    voters[1].vote(json!(null)).await;
    voters[2].vote(json!([0])).await;

    let (_, status) = host.get("/api/host/round").await;
    assert_eq!(status["counts"], json!({ "eligible": 4, "signed": 4, "received": 4 }));

    let (code, closed) = host.post("/api/host/round/close", json!({})).await;
    assert_eq!(code, StatusCode::OK, "{closed}");
    assert_eq!(closed["phase"], "tallied");
    assert_eq!(closed["tally"], json!({ "score": [2, 0, 2], "blank": 1 }));
    assert_eq!(closed["counts"], json!({ "eligible": 4, "signed": 4, "received": 4 }));

    // §8: the result on disk is readable with the password and nothing else.
    let (_, files) = host.get("/api/host/tally-files").await;
    let files = files.as_array().unwrap();
    assert_eq!(files.len(), 1);
    let bytes = STANDARD.decode(files[0]["data"].as_str().unwrap()).unwrap();
    let plain = decrypt_tally(&bytes, key.scalar);
    assert_eq!(plain["meeting"], "Vårmöte");
    assert_eq!(plain["round"], "Chair");
    assert_eq!(plain["score"], json!([2, 0, 2]));
    assert_eq!(plain["counts"]["signed"], 4);
    assert_eq!(plain["participants"].as_array().unwrap().len(), 4);

    // Reset, then a second round works.
    assert_eq!(host.delete("/api/host/round").await.0, StatusCode::NO_CONTENT);
    host.start_round("Treasurer", &["Dag", "Eva"], 1).await;
    voters[0].vote(json!([1])).await;
    let (_, closed) = host.post("/api/host/round/close", json!({})).await;
    assert_eq!(closed["tally"], json!({ "score": [0, 1], "blank": 0 }));
    assert_eq!(closed["counts"]["received"], 1);
}

// ── §5: one ballot per voter per round ───────────────────────────────────────

#[tokio::test]
async fn a_voter_gets_one_signature_per_round() {
    let env = TestEnv::start().await;
    let (host, voters, _) = meeting(&env, 1).await;
    host.start_round("R", &["A", "B"], 1).await;

    voters[0].vote(json!([0])).await;
    let (status, body) = voters[0].sign_ballot(json!([1])).await.unwrap_err();
    assert_eq!((status, error_code(&body)), (StatusCode::CONFLICT, "AlreadySigned"));
}

#[tokio::test]
async fn resubmitting_the_same_ballot_counts_once() {
    let env = TestEnv::start().await;
    let (host, voters, _) = meeting(&env, 1).await;
    host.start_round("R", &["A", "B"], 1).await;

    let (prepared, sig) = voters[0].sign_ballot(json!([0])).await.unwrap();
    assert_eq!(voters[0].submit(&prepared, &sig).await.0, StatusCode::NO_CONTENT);
    let (status, body) = voters[0].submit(&prepared, &sig).await;
    assert_eq!((status, error_code(&body)), (StatusCode::CONFLICT, "AlreadyReceived"));
    let (_, s) = host.get("/api/host/round").await;
    assert_eq!(s["counts"]["received"], 1);
}

#[tokio::test]
async fn invalid_choices_are_rejected() {
    let env = TestEnv::start().await;
    let (host, voters, _) = meeting(&env, 4).await;
    host.start_round("R", &["A", "B", "C"], 2).await;
    // The browser never signs these; if one were signed anyway, the server still refuses it.
    // Each attempt uses up one voter's signature, hence one voter per bad choice.
    let bad = [json!([0, 0]), json!([1, 0]), json!([3]), json!([]), json!([0, 1, 2])];
    for (who, choice) in std::iter::once(&host).chain(&voters).zip(bad) {
        let (prepared, sig) = who.sign_ballot(choice.clone()).await.unwrap();
        let (status, body) = who.submit(&prepared, &sig).await;
        assert_eq!((status, error_code(&body)), (StatusCode::BAD_REQUEST, "InvalidBallot"), "{choice}");
    }
}

#[tokio::test]
async fn ballot_from_an_old_round_is_rejected() {
    let env = TestEnv::start().await;
    let (host, voters, _) = meeting(&env, 1).await;
    host.start_round("R1", &["A"], 1).await;
    let (prepared, sig) = voters[0].sign_ballot(json!([0])).await.unwrap();
    host.delete("/api/host/round").await;
    host.start_round("R2", &["A"], 1).await;
    let (status, body) = voters[0].submit(&prepared, &sig).await;
    assert_eq!((status, error_code(&body)), (StatusCode::BAD_REQUEST, "InvalidSignature"));
}

#[tokio::test]
async fn submitting_needs_an_open_round() {
    let env = TestEnv::start().await;
    let (host, voters, _) = meeting(&env, 1).await;
    host.start_round("R", &["A"], 1).await;
    let (prepared, sig) = voters[0].sign_ballot(json!([0])).await.unwrap();
    host.post("/api/host/round/close", json!({})).await;
    let (status, body) = voters[0].submit(&prepared, &sig).await;
    assert_eq!((status, error_code(&body)), (StatusCode::CONFLICT, "VotingClosed"));
}

// ── §7: refresh safety ───────────────────────────────────────────────────────

#[tokio::test]
async fn status_survives_a_refresh() {
    let env = TestEnv::start().await;
    let (host, voters, _) = meeting(&env, 1).await;
    host.start_round("R", &["A"], 1).await;
    let round_id = round_id(&host).await;

    let (_, s) = voters[0].ta_get("/api/status").await;
    assert_eq!(s, json!({ "round": round_id, "signed": false }));
    voters[0].vote(json!([0])).await;

    // A "refreshed" page has nothing but its cookies, which the same client still holds.
    let (_, s) = voters[0].ta_get("/api/status").await;
    assert_eq!(s, json!({ "round": round_id, "signed": true }));
    let (_, m) = voters[0].get("/api/meeting").await;
    assert_eq!(m["phase"], "voting");
}

async fn round_id(b: &Browser) -> Value {
    b.get("/api/meeting").await.1["round"]["id"].clone()
}

#[tokio::test]
async fn lost_ballot_shows_as_signed_greater_than_received() {
    let env = TestEnv::start().await;
    let (host, voters, _) = meeting(&env, 2).await;
    host.start_round("R", &["A"], 1).await;
    voters[0].vote(json!([0])).await;
    let _never_submitted = voters[1].sign_ballot(json!([0])).await.unwrap();
    let (_, closed) = host.post("/api/host/round/close", json!({})).await;
    assert_eq!(closed["counts"], json!({ "eligible": 3, "signed": 2, "received": 1 }));
}

// ── §4: invites and the voter list ───────────────────────────────────────────

#[tokio::test]
async fn invite_links_work_once() {
    let env = TestEnv::start().await;
    let (host, _, _) = meeting(&env, 0).await;
    let invite = host.invite("Anna", false).await;
    env.browser().join(&invite).await;
    let (status, body) = env.browser().open_invite(&invite).await;
    assert_eq!((status, error_code(&body)), (StatusCode::UNAUTHORIZED, "InviteInvalid"));
}

#[tokio::test]
async fn reset_invite_keeps_the_voter_and_kills_the_old_link_and_sessions() {
    let env = TestEnv::start().await;
    let (host, _, _) = meeting(&env, 0).await;
    let invite = host.invite("Anna", false).await;
    let mut old_device = env.browser();
    old_device.join(&invite).await;

    let voter = invite["voter"].as_str().unwrap();
    let (status, fresh) = host.post(&format!("/api/host/voters/{voter}/reset-invite"), json!({})).await;
    assert_eq!(status, StatusCode::OK, "{fresh}");
    assert_eq!(fresh["voter"], invite["voter"], "same voter ID after a reset");

    assert_eq!(error_code(&old_device.get("/api/session").await.1), "SessionExpired");
    assert_eq!(error_code(&old_device.ta_get("/api/status").await.1), "SessionExpired");
    assert_eq!(error_code(&env.browser().open_invite(&invite).await.1), "InviteInvalid");

    let mut new_device = env.browser();
    new_device.join(&fresh).await;
    assert_eq!(new_device.voter.unwrap().to_string(), voter);
}

#[tokio::test]
async fn voter_list_is_frozen_during_a_round() {
    let env = TestEnv::start().await;
    let (host, voters, _) = meeting(&env, 1).await;
    host.start_round("R", &["A"], 1).await;
    let voter = voters[0].voter.unwrap();
    for (status, body) in [
        host.post("/api/host/voters", json!({ "name": "Late", "isHost": false })).await,
        host.post(&format!("/api/host/voters/{voter}/reset-invite"), json!({})).await,
        host.delete(&format!("/api/host/voters/{voter}")).await,
        host.delete("/api/host/voters").await,
    ] {
        assert_eq!((status, error_code(&body)), (StatusCode::CONFLICT, "RoundInProgress"));
    }
}

#[tokio::test]
async fn unclaimed_invites_are_dropped_when_a_round_starts() {
    let env = TestEnv::start().await;
    let (host, _, _) = meeting(&env, 1).await;
    let unused = host.invite("Never", false).await;
    let started = host.start_round("R", &["A"], 1).await;
    assert_eq!(started["counts"]["eligible"], 2);
    let (_, list) = host.get("/api/host/voters").await;
    assert_eq!(list.as_array().unwrap().len(), 2);
    assert_eq!(error_code(&env.browser().open_invite(&unused).await.1), "InviteInvalid");
}

#[tokio::test]
async fn removed_voters_are_logged_out_of_both_services() {
    let env = TestEnv::start().await;
    let (host, voters, _) = meeting(&env, 1).await;
    let voter = voters[0].voter.unwrap();
    assert_eq!(host.delete(&format!("/api/host/voters/{voter}")).await.0, StatusCode::NO_CONTENT);
    assert_eq!(error_code(&voters[0].get("/api/meeting").await.1), "SessionExpired");
    assert_eq!(error_code(&voters[0].ta_get("/api/status").await.1), "SessionExpired");
}

#[tokio::test]
async fn hosts_can_remove_other_hosts_but_not_themselves() {
    let env = TestEnv::start().await;
    let (host, _, _) = meeting(&env, 0).await;
    let co = host.invite("Co-host", true).await;
    let mut co_host = env.browser();
    co_host.join(&co).await;
    assert_eq!(co_host.get("/api/session").await.1["isHost"], true);

    let me = host.voter.unwrap();
    let (status, body) = host.delete(&format!("/api/host/voters/{me}")).await;
    assert_eq!((status, error_code(&body)), (StatusCode::CONFLICT, "CannotRemoveSelf"));
    let other = co_host.voter.unwrap();
    assert_eq!(host.delete(&format!("/api/host/voters/{other}")).await.0, StatusCode::NO_CONTENT);
}

#[tokio::test]
async fn remove_all_keeps_hosts() {
    let env = TestEnv::start().await;
    let (host, voters, _) = meeting(&env, 2).await;
    host.invite("Co-host", true).await;
    assert_eq!(host.delete("/api/host/voters").await.0, StatusCode::NO_CONTENT);
    let (_, list) = host.get("/api/host/voters").await;
    assert!(list.as_array().unwrap().iter().all(|v| v["isHost"] == true));
    assert_eq!(error_code(&voters[0].get("/api/meeting").await.1), "SessionExpired");
}

// ── Auth ─────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn host_endpoints_need_a_host() {
    let env = TestEnv::start().await;
    let (_, voters, _) = meeting(&env, 1).await;
    let (status, body) = voters[0].get("/api/host/voters").await;
    assert_eq!((status, error_code(&body)), (StatusCode::FORBIDDEN, "NotHost"));
    let (status, body) = env.browser().get("/api/host/voters").await;
    assert_eq!((status, error_code(&body)), (StatusCode::UNAUTHORIZED, "NotLoggedIn"));
    assert_eq!(voters[0].post("/api/host/round", json!({})).await.0, StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn session_reports_who_you_are() {
    let env = TestEnv::start().await;
    let (host, voters, _) = meeting(&env, 1).await;
    let (_, s) = host.get("/api/session").await;
    assert_eq!((s["name"].clone(), s["isHost"].clone()), (json!("Host"), json!(true)));
    let (_, s) = voters[0].get("/api/session").await;
    assert_eq!((s["name"].clone(), s["isHost"].clone()), (json!("Voter 0"), json!(false)));
}

#[tokio::test]
async fn logout_ends_only_this_session() {
    let env = TestEnv::start().await;
    let (host, voters, _) = meeting(&env, 1).await;
    assert_eq!(voters[0].post("/api/logout", json!({})).await.0, StatusCode::NO_CONTENT);
    assert_eq!(error_code(&voters[0].get("/api/meeting").await.1), "NotLoggedIn");
    assert_eq!(host.get("/api/meeting").await.0, StatusCode::OK);
}

#[tokio::test]
async fn closing_the_meeting_ends_everything() {
    let env = TestEnv::start().await;
    let (host, voters, _) = meeting(&env, 1).await;
    assert_eq!(host.delete("/api/host/meeting").await.0, StatusCode::NO_CONTENT);
    assert_eq!(error_code(&voters[0].get("/api/meeting").await.1), "SessionExpired");
    assert_eq!(error_code(&voters[0].ta_get("/api/status").await.1), "SessionExpired");
}

#[tokio::test]
async fn interrupted_trustauth_login_can_get_a_new_ticket() {
    let env = TestEnv::start().await;
    let (host, _, _) = meeting(&env, 0).await;
    let invite = host.invite("Anna", false).await;
    let link = reqwest::Url::parse(invite["inviteLink"].as_str().unwrap()).unwrap();
    let q = |k: &str| link.query_pairs().find(|(key, _)| key == k).unwrap().1.into_owned();

    let b = env.browser();
    let (status, _) = b.post("/api/login", json!({ "meeting": q("meeting"), "invite": q("invite") })).await;
    assert_eq!(status, StatusCode::OK);
    // …the page died before logging in to trustauth.
    let (_, t) = b.post("/api/trustauth-ticket", json!({})).await;
    assert_eq!(b.ta_post("/api/login", json!({ "ticket": t["ticket"] })).await.0, StatusCode::NO_CONTENT);
    assert_eq!(b.ta_get("/api/status").await.0, StatusCode::OK);
}

// ── Failures ─────────────────────────────────────────────────────────────────

#[tokio::test]
async fn trustauth_down_changes_nothing() {
    let mut env = TestEnv::start().await;
    let (host, _, _) = meeting(&env, 0).await;
    let invite = host.invite("Anna", false).await;
    env.stop_trustauth();
    tokio::time::sleep(Duration::from_millis(50)).await;

    let (status, body) = env.browser().open_invite(&invite).await;
    assert_eq!((status, error_code(&body)), (StatusCode::BAD_GATEWAY, "TrustauthUnavailable"));
    let (status, _) = host.post("/api/host/round", json!({ "name": "R", "candidates": ["A"], "maxChoices": 1 })).await;
    assert_eq!(status, StatusCode::BAD_GATEWAY);
    let (_, m) = host.get("/api/meeting").await;
    assert_eq!(m["phase"], "idle", "a failed start leaves the meeting idle");
    let (_, list) = host.get("/api/host/voters").await;
    assert_eq!(list.as_array().unwrap().len(), 2, "and doesn't drop unclaimed invites");

    let (status, body) = env
        .browser()
        .post("/api/meetings", json!({ "title": "T", "hostName": "H", "tallyKey": common::TallyKey::derive("pw").body() }))
        .await;
    assert_eq!((status, error_code(&body)), (StatusCode::BAD_GATEWAY, "TrustauthUnavailable"));
}

#[tokio::test]
async fn failed_tally_write_keeps_the_round_open() {
    // A file where the meetings directory should be: every write fails.
    let blocker = std::env::temp_dir().join(format!("rustsystem-blocker-{}", uuid::Uuid::new_v4()));
    std::fs::write(&blocker, b"").unwrap();
    let env = TestEnv::start_with(|s| s.meetings_dir = blocker.clone()).await;
    let (host, voters, _) = meeting(&env, 1).await;
    host.start_round("R", &["A"], 1).await;
    voters[0].vote(json!([0])).await;

    let (status, body) = host.post("/api/host/round/close", json!({})).await;
    assert_eq!((status, error_code(&body)), (StatusCode::INTERNAL_SERVER_ERROR, "Internal"));
    let (_, m) = host.get("/api/meeting").await;
    assert_eq!(m["phase"], "voting");
    assert_eq!(m["round"]["received"], 1, "the counted ballot is still there");
    host.vote(json!([0])).await;
    std::fs::remove_file(blocker).unwrap();
}

// ── Inputs and responses ─────────────────────────────────────────────────────

#[tokio::test]
async fn inputs_are_validated() {
    let env = TestEnv::start().await;
    let (host, _, _) = meeting(&env, 0).await;
    let long = "x".repeat(81);
    for body in [
        json!({ "name": long, "isHost": false }),
        json!({ "name": "  ", "isHost": false }),
        json!({ "name": "Host", "isHost": false }),
    ] {
        let (status, _) = host.post("/api/host/voters", body).await;
        assert!(status == StatusCode::BAD_REQUEST || status == StatusCode::CONFLICT);
    }
    for body in [
        json!({ "name": "R", "candidates": [], "maxChoices": 1 }),
        json!({ "name": "R", "candidates": ["A", "A"], "maxChoices": 1 }),
        json!({ "name": "R", "candidates": ["A"], "maxChoices": 0 }),
        json!({ "name": "R", "candidates": ["A"], "maxChoices": 2 }),
        json!({ "name": "R", "candidates": ["A"], "maxChoices": 1, "extra": true }),
    ] {
        let (status, body) = host.post("/api/host/round", body.clone()).await;
        assert_eq!((status, error_code(&body)), (StatusCode::BAD_REQUEST, "InvalidInput"));
    }
    let (_, config) = host.get("/api/config").await;
    assert_eq!(config["trustauthUrl"], env.trustauth.as_str());
    assert_eq!(config["maxNameLength"], 80);
    assert_eq!(config["maxLabelLength"], 120);
    assert_eq!(config["maxCandidates"], 100);
}

#[tokio::test]
async fn unknown_api_paths_are_json_404_and_others_serve_the_app() {
    let env = TestEnv::start().await;
    let b = env.browser();
    let (status, body) = b.get("/api/nope").await;
    assert_eq!((status, error_code(&body)), (StatusCode::NOT_FOUND, "NotFound"));
    let page = b.http.get(format!("{}/meeting", env.server)).send().await.unwrap();
    assert_eq!(page.status(), StatusCode::OK);
    assert!(page.text().await.unwrap().contains("Rustsystem"));
}

#[tokio::test]
async fn cookies_are_httponly_and_strict() {
    let env = TestEnv::start().await;
    let res = reqwest::Client::new()
        .post(format!("{}/api/meetings", env.server))
        .json(&json!({ "title": "T", "hostName": "H", "tallyKey": common::TallyKey::derive("pw").body() }))
        .send()
        .await
        .unwrap();
    let cookie = res.headers()["set-cookie"].to_str().unwrap().to_owned();
    assert!(cookie.starts_with("rs_session="));
    for attr in ["HttpOnly", "SameSite=Strict", "Path=/", "Max-Age=43200"] {
        assert!(cookie.contains(attr), "{attr} missing from {cookie}");
    }
}

// ── §5.6: live updates ───────────────────────────────────────────────────────

#[tokio::test]
async fn event_stream_sends_counters_and_ends_when_the_meeting_closes() {
    let env = TestEnv::start().await;
    let (host, voters, _) = meeting(&env, 1).await;
    let mut events = voters[0]
        .http
        .get(format!("{}/api/meeting/events", env.server))
        .send()
        .await
        .unwrap();
    assert_eq!(events.headers()["content-type"], "text/event-stream");

    // Each chunk is one `data: {...}` event (keep-alives are 15 s apart, so none arrive here).
    let mut next = async || -> Option<Value> {
        let chunk = tokio::time::timeout(Duration::from_secs(5), events.chunk()).await.unwrap().unwrap()?;
        let text = String::from_utf8_lossy(&chunk).into_owned();
        let data = text.lines().find_map(|l| l.strip_prefix("data: "))?;
        Some(serde_json::from_str(data).unwrap())
    };

    let first = next().await.unwrap(); // the current counters, on connect
    host.start_round("R", &["A"], 1).await;
    let opened = next().await.unwrap();
    assert!(opened["round"].as_u64() > first["round"].as_u64(), "opening a round moves `round`");

    voters[0].vote(json!([0])).await;
    let voted = next().await.unwrap();
    assert!(voted["version"].as_u64() > opened["version"].as_u64(), "a ballot moves `version`");
    assert_eq!(voted["round"], opened["round"], "but not `round`, so voters' pages don't refetch");

    host.delete("/api/host/meeting").await;
    while next().await.is_some() {}
}

// ── Agenda and attendance (§4.5) ─────────────────────────────────────────────

#[tokio::test]
async fn agenda_and_attendance() {
    let env = TestEnv::start().await;
    let (host, voters, _) = meeting(&env, 2).await;
    let voter = &voters[0];

    let (_, view) = voter.get("/api/meeting").await;
    assert_eq!(view["agenda"], Value::Null);
    let (status, body) = host.put("/api/host/agenda/current", json!({ "index": 0 })).await;
    assert_eq!((status, error_code(&body)), (StatusCode::CONFLICT, "NoAgenda"));

    let agenda = "# Opening\nWelcome.\n## Election of chair\n# Closing\n";
    let (status, body) = host.put("/api/host/agenda", json!({ "markdown": agenda })).await;
    assert_eq!(status, StatusCode::OK, "set agenda: {body}");
    let (_, source) = host.get("/api/host/agenda").await;
    assert_eq!(source["source"], agenda);

    // Voters see the same agenda and follow the host forward and back.
    let (_, before) = voter.get("/api/meeting").await;
    host.put("/api/host/agenda/current", json!({ "index": 2 })).await;
    let (status, _) = host.put("/api/host/agenda/current", json!({ "index": 1 })).await;
    assert_eq!(status, StatusCode::OK);
    let (_, view) = voter.get("/api/meeting").await;
    assert_eq!(view["agenda"]["current"], 1);
    assert_eq!(view["agenda"]["points"][0], json!({ "level": 1, "title": "Opening", "body": "Welcome." }));
    assert!(view["agendaVersion"].as_u64() > before["agendaVersion"].as_u64(), "voter pages refetch on agenda changes");
    let (status, body) = host.put("/api/host/agenda/current", json!({ "index": 3 })).await;
    assert_eq!((status, error_code(&body)), (StatusCode::BAD_REQUEST, "InvalidInput"));

    // Attendance: everyone logged in, then again after a removal.
    let (status, first) = host.post("/api/host/attendance", json!({})).await;
    assert_eq!(status, StatusCode::CREATED);
    assert_eq!(first["point"], json!({ "index": 1, "title": "Election of chair" }));
    assert_eq!(first["present"].as_array().unwrap().len(), 3);
    host.delete(&format!("/api/host/voters/{}", voters[1].voter.unwrap())).await;
    host.post("/api/host/attendance", json!({})).await;

    let (_, log) = host.get("/api/host/attendance").await;
    assert_eq!(log["meeting"], "Vårmöte");
    let counts: Vec<_> = log["records"].as_array().unwrap().iter().map(|r| r["present"].as_array().unwrap().len()).collect();
    assert_eq!(counts, [3, 2]);

    // Hosts only.
    for (status, body) in [
        voter.get("/api/host/agenda").await,
        voter.put("/api/host/agenda", json!({ "markdown": "# X" })).await,
        voter.put("/api/host/agenda/current", json!({ "index": 0 })).await,
        voter.post("/api/host/attendance", json!({})).await,
        voter.get("/api/host/attendance").await,
        voter.delete("/api/host/agenda").await,
    ] {
        assert_eq!((status, error_code(&body)), (StatusCode::FORBIDDEN, "NotHost"));
    }

    let (status, _) = host.delete("/api/host/agenda").await;
    assert_eq!(status, StatusCode::NO_CONTENT);
    assert_eq!(voter.get("/api/meeting").await.1["agenda"], Value::Null);
}
