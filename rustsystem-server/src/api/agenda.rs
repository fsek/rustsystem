//! The agenda and attendance (`docs/PROTOCOL.md` §4.5). Everyone sees the agenda through
//! `GET /api/meeting`; only hosts change it, move through it, and take attendance.

use axum::http::StatusCode;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use tracing::info;

use rustsystem_core::{ApiResult, ErrorCode, extract::Json};

use crate::{
    agenda::Point,
    auth::Host,
    state::{Attendance, MeetingState},
};

/// What every page shows of the agenda.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AgendaView {
    pub points: Vec<Point>,
    /// Index into `points`.
    pub current: usize,
}

pub fn agenda_view(state: &MeetingState) -> Option<AgendaView> {
    state.agenda().map(|(a, current)| AgendaView { points: a.points.clone(), current })
}

#[derive(Serialize)]
pub struct AgendaSource {
    source: String,
}

/// The Markdown, for the editor.
pub async fn source(Host(host): Host) -> ApiResult<Json<AgendaSource>> {
    let state = host.meeting.lock().await;
    let (agenda, _) = state.agenda().ok_or(ErrorCode::NoAgenda)?;
    Ok(Json(AgendaSource { source: agenda.source.clone() }))
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SetAgenda {
    markdown: String,
}

pub async fn set(Host(host): Host, Json(req): Json<SetAgenda>) -> ApiResult<Json<AgendaView>> {
    let mut state = host.meeting.lock().await;
    state.set_agenda(&req.markdown)?;
    let view = agenda_view(&state).expect("just set");
    drop(state);
    host.meeting.agenda_changed();
    info!(meeting = %host.meeting.id, points = view.points.len(), current = view.current, "Agenda set");
    Ok(Json(view))
}

pub async fn clear(Host(host): Host) -> StatusCode {
    let cleared = host.meeting.lock().await.clear_agenda();
    if cleared {
        host.meeting.agenda_changed();
        info!(meeting = %host.meeting.id, "Agenda removed");
    }
    StatusCode::NO_CONTENT
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GoTo {
    index: usize,
}

pub async fn go_to(Host(host): Host, Json(req): Json<GoTo>) -> ApiResult<Json<AgendaView>> {
    let mut state = host.meeting.lock().await;
    state.go_to(req.index)?;
    let view = agenda_view(&state).expect("go_to needs an agenda");
    drop(state);
    host.meeting.agenda_changed();
    info!(meeting = %host.meeting.id, current = req.index, "Agenda point changed");
    Ok(Json(view))
}

// ── Attendance ───────────────────────────────────────────────────────────────

pub async fn take_attendance(Host(host): Host) -> (StatusCode, Json<Attendance>) {
    let record = host.meeting.lock().await.take_attendance().clone();
    host.meeting.changed();
    info!(
        meeting = %host.meeting.id,
        point = ?record.point.as_ref().map(|p| p.index),
        present = record.present.len(),
        "Attendance taken"
    );
    (StatusCode::CREATED, Json(record))
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AttendanceLog {
    meeting: String,
    /// RFC 3339.
    exported_at: String,
    /// Oldest first.
    records: Vec<Attendance>,
}

pub async fn attendance_log(Host(host): Host) -> Json<AttendanceLog> {
    let records = host.meeting.lock().await.attendance().to_vec();
    Json(AttendanceLog { meeting: host.meeting.title.clone(), exported_at: Utc::now().to_rfc3339(), records })
}
