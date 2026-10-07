//! Test harness: the real server and the real trustauth, each on a random localhost port,
//! talking plain HTTP (production uses mTLS between them). A [`Browser`] is one person's
//! browser, with its own cookie jar, doing what the frontend does.
#![allow(dead_code)]

use std::path::PathBuf;

use argon2::{Algorithm, Argon2, Params, Version};
use reqwest::{Client, Response, StatusCode};
use serde_json::{Value, json};
use tokio::{net::TcpListener, task::JoinHandle};
use uuid::Uuid;
use x25519_dalek::{X25519_BASEPOINT_BYTES, x25519};

use rustsystem_core::{
    blind::{RoundPublicKey, client},
    secret::{b64_decode, b64_encode, new_token},
};
use rustsystem_server::{AppState, RateLimits, Settings, router, trustauth::Trustauth};

pub const PASSWORD: &str = "correct horse battery staple";
/// Cheap Argon2id parameters so tests are fast; production uses t=3, m=64 MiB.
pub const T_COST: u32 = 1;
pub const M_COST_KIB: u32 = 8192;

pub struct TestEnv {
    pub server: String,
    pub trustauth: String,
    pub trustauth_internal: String,
    pub meetings_dir: PathBuf,
    pub frontend_dir: PathBuf,
    tasks: Vec<JoinHandle<()>>,
}

async fn serve(router: axum::Router) -> (String, JoinHandle<()>) {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let url = format!("http://{}", listener.local_addr().unwrap());
    let task = tokio::spawn(async move {
        axum::serve(listener, router.into_make_service_with_connect_info::<std::net::SocketAddr>())
            .await
            .unwrap();
    });
    (url, task)
}

fn temp_dir(label: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("rustsystem-{label}-{}", Uuid::new_v4()));
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

impl TestEnv {
    pub async fn start() -> Self {
        Self::start_with(|_| {}).await
    }

    /// `tweak` may change the server's settings, e.g. to make tally writes fail.
    pub async fn start_with(tweak: impl FnOnce(&mut Settings)) -> Self {
        let ta = rustsystem_trustauth::AppState::new(false);
        let (trustauth_internal, t1) = serve(rustsystem_trustauth::internal_router(ta.clone())).await;

        // The server's URL is needed for trustauth's CORS list, so bind it first.
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let server = format!("http://{}", listener.local_addr().unwrap());

        let origin = server.parse().unwrap();
        let (trustauth, t2) = serve(rustsystem_trustauth::public_router(ta, vec![origin], None)).await;

        let meetings_dir = temp_dir("meetings");
        let frontend_dir = temp_dir("frontend");
        std::fs::write(frontend_dir.join("index.html"), "<!doctype html><title>Rustsystem</title>").unwrap();

        let mut settings = Settings {
            public_url: server.clone(),
            trustauth_public_url: trustauth.clone(),
            secure_cookies: false,
            meetings_dir: meetings_dir.clone(),
        };
        tweak(&mut settings);
        let app = AppState::new(settings, Trustauth::new(Client::new(), &trustauth_internal));
        let service = router(app, &frontend_dir, RateLimits::default());
        let t3 = tokio::spawn(async move {
            axum::serve(listener, service.into_make_service_with_connect_info::<std::net::SocketAddr>())
                .await
                .unwrap();
        });

        Self { server, trustauth, trustauth_internal, meetings_dir, frontend_dir, tasks: vec![t1, t2, t3] }
    }

    pub fn browser(&self) -> Browser {
        Browser {
            http: Client::builder().cookie_store(true).build().unwrap(),
            server: self.server.clone(),
            trustauth: self.trustauth.clone(),
            meeting: None,
            voter: None,
        }
    }

    /// Stops trustauth, to test what happens when it's unreachable.
    pub fn stop_trustauth(&mut self) {
        for t in self.tasks.drain(..2) {
            t.abort();
        }
    }
}

impl Drop for TestEnv {
    fn drop(&mut self) {
        for t in &self.tasks {
            t.abort();
        }
        let _ = std::fs::remove_dir_all(&self.meetings_dir);
        let _ = std::fs::remove_dir_all(&self.frontend_dir);
    }
}

/// What the host's browser derives from the meeting password (`frontend/src/utils/cryptoGen.ts`).
pub struct TallyKey {
    pub scalar: [u8; 32],
    pub salt: [u8; 16],
}

impl TallyKey {
    pub fn derive(password: &str) -> Self {
        let salt: [u8; 16] = rand::random();
        let mut scalar = [0u8; 32];
        Argon2::new(Algorithm::Argon2id, Version::V0x13, Params::new(M_COST_KIB, T_COST, 1, Some(32)).unwrap())
            .hash_password_into(password.as_bytes(), &salt, &mut scalar)
            .unwrap();
        Self { scalar, salt }
    }

    pub fn body(&self) -> Value {
        json!({
            "publicKey": b64_encode(x25519(self.scalar, X25519_BASEPOINT_BYTES)),
            "salt": b64_encode(self.salt),
            "tCost": T_COST,
            "mCostKib": M_COST_KIB,
            "pCost": 1,
        })
    }
}

pub struct Browser {
    pub http: Client,
    pub server: String,
    pub trustauth: String,
    pub meeting: Option<Uuid>,
    pub voter: Option<Uuid>,
}

/// A response's status and JSON body (`Null` if there is none).
pub async fn parts(res: Response) -> (StatusCode, Value) {
    let status = res.status();
    let text = res.text().await.unwrap();
    (status, serde_json::from_str(&text).unwrap_or(Value::Null))
}

pub fn error_code(body: &Value) -> &str {
    body["code"].as_str().unwrap_or("")
}

impl Browser {
    pub async fn get(&self, path: &str) -> (StatusCode, Value) {
        parts(self.http.get(format!("{}{path}", self.server)).send().await.unwrap()).await
    }

    pub async fn post(&self, path: &str, body: Value) -> (StatusCode, Value) {
        parts(self.http.post(format!("{}{path}", self.server)).json(&body).send().await.unwrap()).await
    }

    pub async fn delete(&self, path: &str) -> (StatusCode, Value) {
        parts(self.http.delete(format!("{}{path}", self.server)).send().await.unwrap()).await
    }

    pub async fn ta_get(&self, path: &str) -> (StatusCode, Value) {
        parts(self.http.get(format!("{}{path}", self.trustauth)).send().await.unwrap()).await
    }

    pub async fn ta_post(&self, path: &str, body: Value) -> (StatusCode, Value) {
        parts(self.http.post(format!("{}{path}", self.trustauth)).json(&body).send().await.unwrap()).await
    }

    async fn trustauth_login(&mut self, body: &Value) {
        self.meeting = Some(body["meeting"].as_str().unwrap().parse().unwrap());
        self.voter = Some(body["voter"].as_str().unwrap().parse().unwrap());
        let (status, err) = self.ta_post("/api/login", json!({ "ticket": body["ticket"] })).await;
        assert_eq!(status, StatusCode::NO_CONTENT, "trustauth login: {err}");
    }

    /// Creates a meeting with `key` as its tally key and logs in to both services as host.
    pub async fn create_meeting_with_key(&mut self, title: &str, host: &str, key: &TallyKey) -> Value {
        let (status, body) = self
            .post("/api/meetings", json!({ "title": title, "hostName": host, "tallyKey": key.body() }))
            .await;
        assert_eq!(status, StatusCode::CREATED, "create meeting: {body}");
        self.trustauth_login(&body).await;
        body
    }

    pub async fn create_meeting(&mut self, title: &str, host: &str) -> TallyKey {
        let key = TallyKey::derive(PASSWORD);
        self.create_meeting_with_key(title, host, &key).await;
        key
    }

    /// Host: invites someone and returns the invite (`{voter, inviteLink, qrSvg}`).
    pub async fn invite(&self, name: &str, is_host: bool) -> Value {
        let (status, body) = self.post("/api/host/voters", json!({ "name": name, "isHost": is_host })).await;
        assert_eq!(status, StatusCode::CREATED, "invite: {body}");
        body
    }

    /// Opens an invite link the way `/login` does. Returns the server's login response.
    pub async fn open_invite(&mut self, invite: &Value) -> (StatusCode, Value) {
        let link = reqwest::Url::parse(invite["inviteLink"].as_str().unwrap()).unwrap();
        let q = |k: &str| link.query_pairs().find(|(key, _)| key == k).unwrap().1.into_owned();
        let (status, body) = self.post("/api/login", json!({ "meeting": q("meeting"), "invite": q("invite") })).await;
        if status == StatusCode::OK {
            self.trustauth_login(&body).await;
        }
        (status, body)
    }

    pub async fn join(&mut self, invite: &Value) {
        let (status, body) = self.open_invite(invite).await;
        assert_eq!(status, StatusCode::OK, "login: {body}");
    }

    /// Host: opens a round.
    pub async fn start_round(&self, name: &str, candidates: &[&str], max_choices: usize) -> Value {
        let (status, body) = self
            .post("/api/host/round", json!({ "name": name, "candidates": candidates, "maxChoices": max_choices }))
            .await;
        assert_eq!(status, StatusCode::CREATED, "start round: {body}");
        body
    }

    /// Steps 1–2 of voting: build the ballot and get trustauth's blind signature on it.
    /// Returns `(prepared, sig)` ready to submit, or trustauth's error.
    pub async fn sign_ballot(&self, choice: Value) -> Result<(Vec<u8>, Vec<u8>), (StatusCode, Value)> {
        let (_, meeting) = self.get("/api/meeting").await;
        let round = &meeting["round"];
        let pk = RoundPublicKey::from_der(&b64_decode(round["publicKey"].as_str().unwrap()).unwrap()).unwrap();
        let msg = json!({ "v": 1, "round": round["id"], "choice": choice, "nonce": new_token() }).to_string();

        let blinded = client::blind(&pk, msg.as_bytes()).unwrap();
        let (status, body) = self
            .ta_post("/api/sign", json!({ "round": round["id"], "blinded": b64_encode(&blinded.blinded) }))
            .await;
        if status != StatusCode::OK {
            return Err((status, body));
        }
        let blind_sig = b64_decode(body["blind_sig"].as_str().unwrap()).unwrap();
        Ok(client::finalize(&pk, &blinded, &blind_sig).unwrap())
    }

    /// Step 3: submit anonymously, from a client with no cookies at all.
    pub async fn submit(&self, prepared: &[u8], sig: &[u8]) -> (StatusCode, Value) {
        let anonymous = Client::new();
        let body = json!({ "meeting": self.meeting, "prepared": b64_encode(prepared), "sig": b64_encode(sig) });
        parts(anonymous.post(format!("{}/api/ballot", self.server)).json(&body).send().await.unwrap()).await
    }

    pub async fn vote(&self, choice: Value) {
        let (prepared, sig) = self.sign_ballot(choice).await.expect("signing");
        let (status, body) = self.submit(&prepared, &sig).await;
        assert_eq!(status, StatusCode::NO_CONTENT, "submit: {body}");
    }
}
