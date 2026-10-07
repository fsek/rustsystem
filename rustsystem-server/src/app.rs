//! Process-wide state: the meetings, the sessions, and the live-update plumbing.
//!
//! Locking is deliberately simple:
//! - the meeting registry and the session table are each behind a plain mutex that is only held
//!   for a map lookup or update, never across an `.await`;
//! - each meeting's [`MeetingState`] is behind its own async mutex. Handlers hold it for the
//!   whole of an operation, including the trustauth call when a round opens or closes, so every
//!   operation on a meeting is atomic. With one lock per meeting there is no lock order to get
//!   wrong.

use std::{
    collections::HashMap,
    path::PathBuf,
    sync::{
        Arc, Mutex, MutexGuard,
        atomic::{AtomicBool, AtomicUsize, Ordering},
    },
    time::{Duration, Instant},
};

use tokio::sync::watch;
use tracing::info;

use rustsystem_core::{
    ApiResult, ErrorCode,
    internal::{MeetingId, VoterId},
    secret::{TokenHash, hash_token, new_token},
    session::SESSION_TTL,
};

use crate::{state::MeetingState, tally::TallyKey, trustauth::Trustauth};

/// Meetings are removed this long after they were created.
pub const MEETING_TTL: Duration = SESSION_TTL;
/// Open live-update streams allowed per meeting and in total.
pub const MAX_SSE_PER_MEETING: usize = 512;
pub const MAX_SSE_TOTAL: usize = 4096;

#[derive(Clone, Debug)]
pub struct Settings {
    /// Where browsers reach the server; used in invite links.
    pub public_url: String,
    /// Where browsers reach trustauth; told to the frontend by `GET /api/config`.
    pub trustauth_public_url: String,
    pub secure_cookies: bool,
    /// Tally files go in `<meetings_dir>/<meeting id>/`.
    pub meetings_dir: PathBuf,
}

/// Change counters sent on the event stream. `round` changes only when a round opens, closes or
/// is reset (or the meeting closes), and `agenda` when the agenda or its current point changes —
/// all a voter's page cares about. `version` changes on every change, including each ballot and
/// login, which only hosts' pages track.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, serde::Serialize)]
pub struct Versions {
    pub version: u64,
    pub round: u64,
    pub agenda: u64,
}

pub struct Meeting {
    pub id: MeetingId,
    pub title: String,
    pub created_at: Instant,
    pub tally_key: TallyKey,
    state: tokio::sync::Mutex<MeetingState>,
    /// Live-update streams send these; clients refetch when they change.
    versions: watch::Sender<Versions>,
    closed: AtomicBool,
    sse_connections: AtomicUsize,
}

impl Meeting {
    pub fn new(id: MeetingId, title: String, tally_key: TallyKey, state: MeetingState) -> Self {
        Self {
            id,
            title,
            created_at: Instant::now(),
            tally_key,
            state: tokio::sync::Mutex::new(state),
            versions: watch::Sender::new(Versions::default()),
            closed: AtomicBool::new(false),
            sse_connections: AtomicUsize::new(0),
        }
    }

    pub async fn lock(&self) -> tokio::sync::MutexGuard<'_, MeetingState> {
        self.state.lock().await
    }

    /// Something changed that hosts' pages show (a ballot, a login, the voter list).
    pub fn changed(&self) {
        self.versions.send_modify(|v| v.version += 1);
    }

    /// A round opened, closed or was reset: every page refetches.
    pub fn round_changed(&self) {
        self.versions.send_modify(|v| {
            v.version += 1;
            v.round += 1;
        });
    }

    /// The agenda or its current point changed: every page refetches.
    pub fn agenda_changed(&self) {
        self.versions.send_modify(|v| {
            v.version += 1;
            v.agenda += 1;
        });
    }

    pub fn versions(&self) -> Versions {
        *self.versions.borrow()
    }

    pub fn subscribe(&self) -> watch::Receiver<Versions> {
        self.versions.subscribe()
    }

    pub fn is_closed(&self) -> bool {
        self.closed.load(Ordering::Relaxed)
    }

    pub fn sse_connections(&self) -> usize {
        self.sse_connections.load(Ordering::Relaxed)
    }
}

struct Session {
    meeting: MeetingId,
    voter: VoterId,
    expires: Instant,
}

struct Inner {
    settings: Settings,
    trustauth: Trustauth,
    meetings: Mutex<HashMap<MeetingId, Arc<Meeting>>>,
    sessions: Mutex<HashMap<TokenHash, Session>>,
    sse_total: AtomicUsize,
}

#[derive(Clone)]
pub struct AppState(Arc<Inner>);

fn lock<T>(m: &Mutex<T>) -> MutexGuard<'_, T> {
    m.lock().unwrap_or_else(|poisoned| poisoned.into_inner())
}

impl AppState {
    pub fn new(settings: Settings, trustauth: Trustauth) -> Self {
        Self(Arc::new(Inner {
            settings,
            trustauth,
            meetings: Mutex::default(),
            sessions: Mutex::default(),
            sse_total: AtomicUsize::new(0),
        }))
    }

    pub fn settings(&self) -> &Settings {
        &self.0.settings
    }

    pub fn trustauth(&self) -> &Trustauth {
        &self.0.trustauth
    }

    pub fn meeting_dir(&self, meeting: MeetingId) -> PathBuf {
        self.0.settings.meetings_dir.join(meeting.to_string())
    }

    // ── Meetings ─────────────────────────────────────────────────────────────

    pub fn insert_meeting(&self, meeting: Meeting) -> Arc<Meeting> {
        let meeting = Arc::new(meeting);
        lock(&self.0.meetings).insert(meeting.id, meeting.clone());
        meeting
    }

    pub fn meeting(&self, id: MeetingId) -> Option<Arc<Meeting>> {
        lock(&self.0.meetings).get(&id).cloned()
    }

    /// Removes a meeting and every session in it, and ends its live-update streams.
    pub fn remove_meeting(&self, id: MeetingId) -> Option<Arc<Meeting>> {
        let meeting = lock(&self.0.meetings).remove(&id)?;
        lock(&self.0.sessions).retain(|_, s| s.meeting != id);
        meeting.closed.store(true, Ordering::Relaxed);
        meeting.round_changed();
        Some(meeting)
    }

    // ── Sessions ─────────────────────────────────────────────────────────────

    /// Starts a session and returns the token for the cookie.
    pub fn create_session(&self, meeting: MeetingId, voter: VoterId) -> String {
        let token = new_token();
        let session = Session { meeting, voter, expires: Instant::now() + SESSION_TTL };
        lock(&self.0.sessions).insert(hash_token(&token), session);
        token
    }

    pub fn session(&self, token: &str) -> Option<(MeetingId, VoterId)> {
        let sessions = lock(&self.0.sessions);
        let s = sessions.get(&hash_token(token))?;
        (s.expires > Instant::now()).then_some((s.meeting, s.voter))
    }

    pub fn end_session(&self, token: &str) {
        lock(&self.0.sessions).remove(&hash_token(token));
    }

    /// Logs a voter out on every device (they were removed, or their invite was reset).
    pub fn end_voter_sessions(&self, meeting: MeetingId, voter: VoterId) {
        lock(&self.0.sessions).retain(|_, s| !(s.meeting == meeting && s.voter == voter));
    }

    // ── Live-update streams ──────────────────────────────────────────────────

    /// Reserves a live-update stream for `meeting`; dropping the slot releases it.
    pub fn acquire_sse_slot(&self, meeting: &Arc<Meeting>) -> ApiResult<SseSlot> {
        if self.0.sse_total.fetch_add(1, Ordering::AcqRel) >= MAX_SSE_TOTAL {
            self.0.sse_total.fetch_sub(1, Ordering::AcqRel);
            return Err(ErrorCode::TooManyConnections.into());
        }
        if meeting.sse_connections.fetch_add(1, Ordering::AcqRel) >= MAX_SSE_PER_MEETING {
            meeting.sse_connections.fetch_sub(1, Ordering::AcqRel);
            self.0.sse_total.fetch_sub(1, Ordering::AcqRel);
            return Err(ErrorCode::TooManyConnections.into());
        }
        Ok(SseSlot { app: self.clone(), meeting: meeting.clone() })
    }

    pub fn sse_total(&self) -> usize {
        self.0.sse_total.load(Ordering::Relaxed)
    }

    // ── Pruning ──────────────────────────────────────────────────────────────

    /// Removes meetings older than [`MEETING_TTL`] and expired sessions. Returns the removed
    /// meetings so the caller can tell trustauth.
    pub fn prune(&self, now: Instant) -> Vec<MeetingId> {
        let stale: Vec<MeetingId> = lock(&self.0.meetings)
            .values()
            .filter(|m| now.duration_since(m.created_at) >= MEETING_TTL)
            .map(|m| m.id)
            .collect();
        for id in &stale {
            self.remove_meeting(*id);
            info!(meeting = %id, "Meeting expired and was removed");
        }
        lock(&self.0.sessions).retain(|_, s| s.expires > now);
        stale
    }

    /// Prunes once a minute.
    pub fn spawn_pruning(&self) {
        let app = self.clone();
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(60));
            loop {
                interval.tick().await;
                for id in app.prune(Instant::now()) {
                    app.trustauth().drop_meeting(id).await;
                }
            }
        });
    }
}

pub struct SseSlot {
    app: AppState,
    meeting: Arc<Meeting>,
}

impl std::fmt::Debug for SseSlot {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "SseSlot({})", self.meeting.id)
    }
}

impl SseSlot {
    pub fn meeting(&self) -> &Arc<Meeting> {
        &self.meeting
    }
}

impl Drop for SseSlot {
    fn drop(&mut self) {
        self.meeting.sse_connections.fetch_sub(1, Ordering::AcqRel);
        self.app.0.sse_total.fetch_sub(1, Ordering::AcqRel);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tally::Kdf;
    use uuid::Uuid;

    fn app() -> AppState {
        let settings = Settings {
            public_url: "http://localhost".into(),
            trustauth_public_url: "http://localhost:2443".into(),
            secure_cookies: false,
            meetings_dir: std::env::temp_dir(),
        };
        AppState::new(settings, Trustauth::new(reqwest::Client::new(), "http://127.0.0.1:9"))
    }

    fn meeting(app: &AppState) -> (Arc<Meeting>, VoterId) {
        let (state, host) = MeetingState::new("Host").unwrap();
        let key = TallyKey { public_key: [9; 32], kdf: Kdf { salt: [0; 16], t_cost: 3, m_cost_kib: 65536, p_cost: 1 } };
        (app.insert_meeting(Meeting::new(Uuid::new_v4(), "M".into(), key, state)), host)
    }

    #[test]
    fn sessions_end_with_voter_or_meeting() {
        let app = app();
        let (m, host) = meeting(&app);
        let other = Uuid::new_v4();
        let (a, b) = (app.create_session(m.id, host), app.create_session(m.id, other));
        assert_eq!(app.session(&a), Some((m.id, host)));

        app.end_voter_sessions(m.id, host);
        assert_eq!(app.session(&a), None);
        assert!(app.session(&b).is_some());

        app.remove_meeting(m.id);
        assert_eq!(app.session(&b), None);
        assert!(m.is_closed());
        assert!(app.meeting(m.id).is_none());
    }

    #[test]
    fn sse_slots_are_capped_per_meeting_and_released() {
        let app = app();
        let (busy, _) = meeting(&app);
        let (quiet, _) = meeting(&app);
        let mut slots: Vec<_> = (0..MAX_SSE_PER_MEETING).map(|_| app.acquire_sse_slot(&busy).unwrap()).collect();
        assert_eq!(app.acquire_sse_slot(&busy).unwrap_err().code, ErrorCode::TooManyConnections);
        assert_eq!(app.sse_total(), MAX_SSE_PER_MEETING, "a refused slot must not leak a global slot");

        let other = app.acquire_sse_slot(&quiet).unwrap();
        assert_eq!(quiet.sse_connections(), 1);

        slots.pop();
        assert!(app.acquire_sse_slot(&busy).is_ok(), "a released slot is reusable");
        drop(other);
        assert_eq!(quiet.sse_connections(), 0);
    }

    #[test]
    fn prune_removes_old_meetings_and_expired_sessions() {
        let app = app();
        let (m, host) = meeting(&app);
        let token = app.create_session(m.id, host);
        assert!(app.prune(Instant::now()).is_empty());
        let removed = app.prune(Instant::now() + MEETING_TTL);
        assert_eq!(removed, vec![m.id]);
        assert_eq!(app.session(&token), None);
    }
}
