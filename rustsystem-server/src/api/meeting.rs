//! Meetings and sessions: creating a meeting, logging in and out, and reading meeting state
//! (`docs/PROTOCOL.md` §4).

use axum::{extract::State, http::StatusCode};
use axum_extra::extract::CookieJar;
use serde::{Deserialize, Serialize};
use tracing::info;
use uuid::Uuid;

use rustsystem_core::{
    ApiResult, ErrorCode,
    extract::Json,
    internal::{MeetingId, RoundId, VoterId},
    session::{SERVER_COOKIE, expired_cookie, session_cookie},
};

use crate::{
    app::{AppState, Meeting},
    auth::{Host, Member},
    state::{MAX_CANDIDATES, MAX_LABEL_LENGTH, MAX_NAME_LENGTH, MeetingState, Phase, clean_text},
    tally::{Kdf, TallyKey},
};

// ── GET /api/config ──────────────────────────────────────────────────────────

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ClientConfig {
    /// Where the browser reaches trustauth. Served at runtime so one frontend build works in
    /// every environment.
    trustauth_url: String,
    max_name_length: usize,
    max_label_length: usize,
    max_candidates: usize,
}

/// What the frontend needs to know about this deployment: where trustauth is, and the input
/// limits (so its `maxLength`s match what the server accepts).
pub async fn config(State(app): State<AppState>) -> Json<ClientConfig> {
    Json(ClientConfig {
        trustauth_url: app.settings().trustauth_public_url.clone(),
        max_name_length: MAX_NAME_LENGTH,
        max_label_length: MAX_LABEL_LENGTH,
        max_candidates: MAX_CANDIDATES,
    })
}

// ── POST /api/meetings ───────────────────────────────────────────────────────

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CreateMeeting {
    title: String,
    host_name: String,
    tally_key: TallyKeyBody,
}

/// The public half of the key the host's browser derived from the meeting password.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct TallyKeyBody {
    /// X25519, base64url.
    public_key: String,
    /// Argon2id parameters, base64url salt.
    salt: String,
    t_cost: u32,
    m_cost_kib: u32,
    p_cost: u8,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LoggedIn {
    meeting: MeetingId,
    voter: VoterId,
    is_host: bool,
    /// Present to trustauth's `POST /api/login` within a minute.
    ticket: String,
}

pub async fn create(
    State(app): State<AppState>,
    jar: CookieJar,
    Json(req): Json<CreateMeeting>,
) -> ApiResult<(StatusCode, CookieJar, Json<LoggedIn>)> {
    let title = clean_text("The meeting title", &req.title, MAX_LABEL_LENGTH)?;
    let k = &req.tally_key;
    let tally_key = TallyKey::new(&k.public_key, Kdf::new(&k.salt, k.t_cost, k.m_cost_kib, k.p_cost)?)?;
    let (state, host) = MeetingState::new(&req.host_name)?;
    let id = Uuid::new_v4();

    // Ask trustauth first: if it's unreachable, no meeting is created.
    let ticket = app.trustauth().issue_ticket(id, host).await?;
    app.insert_meeting(Meeting::new(id, title.clone(), tally_key, state));
    let token = app.create_session(id, host);
    info!(meeting = %id, title = %title, "Meeting created");

    let cookie = session_cookie(SERVER_COOKIE, token, app.settings().secure_cookies);
    let body = LoggedIn { meeting: id, voter: host, is_host: true, ticket };
    Ok((StatusCode::CREATED, jar.add(cookie), Json(body)))
}

// ── POST /api/login ──────────────────────────────────────────────────────────

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Login {
    meeting: MeetingId,
    invite: String,
}

pub async fn login(
    State(app): State<AppState>,
    jar: CookieJar,
    Json(req): Json<Login>,
) -> ApiResult<(CookieJar, Json<LoggedIn>)> {
    let meeting = app.meeting(req.meeting).ok_or(ErrorCode::InviteInvalid)?;
    let (voter, is_host, ticket, name) = {
        let mut state = meeting.lock().await;
        let voter = state.find_invite(&req.invite)?;
        // Trustauth first, then use up the invite: if trustauth is unreachable, the link
        // still works when the voter retries.
        let ticket = app.trustauth().issue_ticket(meeting.id, voter).await?;
        state.claim_invite(voter)?;
        let v = state.voter(voter).ok_or(ErrorCode::VoterNotFound)?;
        (voter, v.is_host, ticket, v.name.clone())
    };
    let token = app.create_session(meeting.id, voter);
    meeting.changed();
    info!(meeting = %meeting.id, voter = %name, is_host, "Voter logged in");

    let cookie = session_cookie(SERVER_COOKIE, token, app.settings().secure_cookies);
    Ok((jar.add(cookie), Json(LoggedIn { meeting: meeting.id, voter, is_host, ticket })))
}

// ── POST /api/logout, POST /api/trustauth-ticket, GET /api/session ───────────

pub async fn logout(State(app): State<AppState>, member: Member, jar: CookieJar) -> (StatusCode, CookieJar) {
    app.end_session(&member.token);
    (StatusCode::NO_CONTENT, jar.add(expired_cookie(SERVER_COOKIE)))
}

#[derive(Serialize)]
pub struct Ticket {
    ticket: String,
}

/// A new trustauth ticket, for when the browser died between the two logins.
pub async fn trustauth_ticket(State(app): State<AppState>, member: Member) -> ApiResult<Json<Ticket>> {
    let ticket = app.trustauth().issue_ticket(member.meeting.id, member.voter).await?;
    Ok(Json(Ticket { ticket }))
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Session {
    meeting: MeetingId,
    voter: VoterId,
    name: String,
    is_host: bool,
}

pub async fn session(member: Member) -> ApiResult<Json<Session>> {
    let state = member.meeting.lock().await;
    let v = state.voter(member.voter).ok_or(ErrorCode::SessionExpired)?;
    Ok(Json(Session {
        meeting: member.meeting.id,
        voter: member.voter,
        name: v.name.clone(),
        is_host: v.is_host,
    }))
}

// ── GET /api/meeting ─────────────────────────────────────────────────────────

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MeetingView {
    title: String,
    /// The same counters the event stream sends.
    version: u64,
    round_version: u64,
    /// Voters who have logged in.
    participants: usize,
    /// `"idle"`, `"voting"` or `"tallied"`.
    phase: &'static str,
    round: Option<RoundView>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RoundView {
    pub id: RoundId,
    pub name: String,
    pub candidates: Vec<String>,
    pub max_choices: usize,
    /// SPKI DER, base64url. Only while voting: it's what ballots are blinded for.
    pub public_key: Option<String>,
    pub eligible: usize,
    pub received: usize,
}

pub fn round_view(phase: &Phase) -> Option<RoundView> {
    match phase {
        Phase::Idle => None,
        Phase::Voting(r) => Some(RoundView {
            id: r.id,
            name: r.spec.name.clone(),
            candidates: r.spec.candidates.clone(),
            max_choices: r.spec.max_choices,
            public_key: Some(r.public_key_b64.clone()),
            eligible: r.eligible,
            received: r.received(),
        }),
        Phase::Tallied(c) => Some(RoundView {
            id: c.id,
            name: c.spec.name.clone(),
            candidates: c.spec.candidates.clone(),
            max_choices: c.spec.max_choices,
            public_key: None,
            eligible: c.counts.eligible,
            received: c.counts.received,
        }),
    }
}

/// Everything a voter's page shows. Clients refetch this whenever the event stream ticks.
pub async fn get(member: Member) -> Json<MeetingView> {
    let state = member.meeting.lock().await;
    let versions = member.meeting.versions();
    Json(MeetingView {
        title: member.meeting.title.clone(),
        version: versions.version,
        round_version: versions.round,
        participants: state.participants(),
        phase: state.phase().name(),
        round: round_view(state.phase()),
    })
}

// ── DELETE /api/host/meeting ─────────────────────────────────────────────────

pub async fn close(State(app): State<AppState>, Host(host): Host, jar: CookieJar) -> (StatusCode, CookieJar) {
    let id = host.meeting.id;
    app.remove_meeting(id);
    app.trustauth().drop_meeting(id).await;
    info!(meeting = %id, "Meeting closed");
    (StatusCode::NO_CONTENT, jar.add(expired_cookie(SERVER_COOKIE)))
}
