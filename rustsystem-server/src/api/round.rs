//! Vote rounds, run by hosts (`docs/PROTOCOL.md` §5): open, watch, close, reset, and download
//! the encrypted results.

use axum::{extract::State, http::StatusCode};
use base64::{Engine, engine::general_purpose::STANDARD};
use serde::{Deserialize, Serialize};
use tracing::info;
use uuid::Uuid;

use rustsystem_core::{ApiResult, ErrorCode, extract::Json, internal::RoundId};

use crate::{
    api::meeting::{RoundView, round_view},
    app::{AppState, Meeting},
    auth::Host,
    state::{Counts, Phase, Round, RoundSpec, Tally},
    tally::{self, TallyFile},
};

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct StartRound {
    name: String,
    candidates: Vec<String>,
    max_choices: usize,
    #[serde(default)]
    shuffle: bool,
}

/// What a host sees about the round: everything voters see, plus the counts and the result.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HostRoundView {
    phase: &'static str,
    round: Option<RoundView>,
    counts: Option<Counts>,
    tally: Option<Tally>,
}

pub async fn start(
    State(app): State<AppState>,
    Host(host): Host,
    Json(req): Json<StartRound>,
) -> ApiResult<(StatusCode, Json<HostRoundView>)> {
    let spec = RoundSpec::new(&req.name, &req.candidates, req.max_choices, req.shuffle)?;
    let meeting = &host.meeting;
    let mut state = meeting.lock().await;

    let eligible = state.eligible_voters()?;
    let id = Uuid::new_v4();
    // The meeting stays locked while trustauth generates the key, so nothing can change
    // the voter list in between. If trustauth fails, nothing has changed.
    let public_key = app.trustauth().start_round(meeting.id, id, eligible.clone()).await?;
    let round = Round::new(id, spec, public_key, eligible.len())?;
    let removed = state.open_round(round)?;

    info!(
        meeting = %meeting.id,
        round = %id,
        eligible = eligible.len(),
        unclaimed_removed = removed.len(),
        "Round opened"
    );
    let mut view = host_view(state.phase());
    drop(state);
    if let Some(counts) = view.counts.as_mut() {
        counts.signed = Some(0);
    }
    meeting.round_changed();
    Ok((StatusCode::CREATED, Json(view)))
}

/// How many voters trustauth has signed this round, or `None` if it can't say right now.
async fn signed_count(app: &AppState, meeting: &Meeting, round: RoundId) -> Option<usize> {
    let status = app.trustauth().round_status(meeting.id).await.ok()?;
    (status.round == Some(round)).then_some(status.signed)
}

fn host_view(phase: &Phase) -> HostRoundView {
    let (counts, tally) = match phase {
        Phase::Idle => (None, None),
        Phase::Voting(r) => (Some(Counts { eligible: r.eligible, signed: None, received: r.received() }), None),
        Phase::Tallied(c) => (Some(c.counts), Some(c.tally.clone())),
    };
    HostRoundView { phase: phase.name(), round: round_view(phase), counts, tally }
}

pub async fn status(State(app): State<AppState>, Host(host): Host) -> Json<HostRoundView> {
    let (mut view, open_round) = {
        let state = host.meeting.lock().await;
        let open = match state.phase() {
            Phase::Voting(r) => Some(r.id),
            _ => None,
        };
        (host_view(state.phase()), open)
    };
    // Ask trustauth after releasing the lock, so a slow trustauth never stalls ballots.
    if let (Some(id), Some(counts)) = (open_round, view.counts.as_mut()) {
        counts.signed = signed_count(&app, &host.meeting, id).await;
    }
    Json(view)
}

pub async fn close(State(app): State<AppState>, Host(host): Host) -> ApiResult<Json<HostRoundView>> {
    let meeting = &host.meeting;
    let mut state = meeting.lock().await;
    let round_id = match state.phase() {
        Phase::Voting(r) => r.id,
        _ => return Err(ErrorCode::VotingClosed.into()),
    };
    let signed = signed_count(&app, meeting, round_id).await;
    let closed = state.tally(signed)?;
    let participants = state.voters().into_iter().map(|(_, v)| v.name.clone()).collect();

    // Write the result before changing state: if the write fails, the round stays open.
    let file = TallyFile::new(&meeting.title, &closed, participants);
    tally::write(&app.meeting_dir(meeting.id), &meeting.tally_key, &file, &closed)?;
    state.finish_close(closed.clone())?;
    let view = host_view(state.phase());
    drop(state);

    app.trustauth().drop_round(meeting.id).await;
    meeting.round_changed();
    info!(
        meeting = %meeting.id,
        round = %closed.id,
        eligible = closed.counts.eligible,
        signed = ?closed.counts.signed,
        received = closed.counts.received,
        "Round closed and tally written"
    );
    Ok(Json(view))
}

/// Back to idle: cancels an open round (its ballots are discarded and no file is written) or
/// clears a closed round's result.
pub async fn reset(State(app): State<AppState>, Host(host): Host) -> StatusCode {
    let was_active = host.meeting.lock().await.reset_round();
    if was_active {
        app.trustauth().drop_round(host.meeting.id).await;
        host.meeting.round_changed();
        info!(meeting = %host.meeting.id, "Round reset");
    }
    StatusCode::NO_CONTENT
}

#[derive(Serialize)]
pub struct TallyFileEntry {
    filename: String,
    /// The encrypted file, standard base64.
    data: String,
}

pub async fn tally_files(State(app): State<AppState>, Host(host): Host) -> ApiResult<Json<Vec<TallyFileEntry>>> {
    let files = tally::read_all(&app.meeting_dir(host.meeting.id))?;
    Ok(Json(
        files
            .into_iter()
            .map(|(filename, bytes)| TallyFileEntry { filename, data: STANDARD.encode(bytes) })
            .collect(),
    ))
}
