//! The voter list, managed by hosts (`docs/PROTOCOL.md` §4.2). Frozen while a round is open.

use axum::{extract::State, http::StatusCode};
use serde::{Deserialize, Serialize};
use tracing::info;

use rustsystem_core::{
    ApiResult,
    extract::{Json, Path},
    internal::VoterId,
};

use crate::{
    app::AppState,
    auth::Host,
    invite::{Invite, invite},
};

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VoterView {
    id: VoterId,
    name: String,
    is_host: bool,
    logged_in: bool,
    /// Unix seconds.
    added_at: u64,
}

pub async fn list(Host(host): Host) -> Json<Vec<VoterView>> {
    let state = host.meeting.lock().await;
    Json(
        state
            .voters()
            .into_iter()
            .map(|(id, v)| VoterView {
                id,
                name: v.name.clone(),
                is_host: v.is_host,
                logged_in: v.logged_in,
                added_at: v.added_at_unix(),
            })
            .collect(),
    )
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AddVoter {
    name: String,
    is_host: bool,
}

pub async fn add(
    State(app): State<AppState>,
    Host(host): Host,
    Json(req): Json<AddVoter>,
) -> ApiResult<(StatusCode, Json<Invite>)> {
    let (voter, secret) = host.meeting.lock().await.add_voter(&req.name, req.is_host)?;
    host.meeting.changed();
    info!(meeting = %host.meeting.id, is_host = req.is_host, "Voter invited");
    let inv = invite(&app.settings().public_url, host.meeting.id, voter, &secret)?;
    Ok((StatusCode::CREATED, Json(inv)))
}

/// A new invite link for the same voter. The old link stops working and the voter is logged
/// out everywhere, but keeps their voter ID.
pub async fn reset_invite(
    State(app): State<AppState>,
    Host(host): Host,
    Path(voter): Path<VoterId>,
) -> ApiResult<Json<Invite>> {
    let secret = host.meeting.lock().await.reset_invite(voter)?;
    app.end_voter_sessions(host.meeting.id, voter);
    app.trustauth().drop_voter(host.meeting.id, voter).await;
    host.meeting.changed();
    info!(meeting = %host.meeting.id, "Invite reset");
    Ok(Json(invite(&app.settings().public_url, host.meeting.id, voter, &secret)?))
}

pub async fn remove(
    State(app): State<AppState>,
    Host(host): Host,
    Path(voter): Path<VoterId>,
) -> ApiResult<StatusCode> {
    let removed = host.meeting.lock().await.remove_voter(host.voter, voter)?;
    app.end_voter_sessions(host.meeting.id, voter);
    app.trustauth().drop_voter(host.meeting.id, voter).await;
    host.meeting.changed();
    info!(meeting = %host.meeting.id, voter = %removed.name, "Voter removed");
    Ok(StatusCode::NO_CONTENT)
}

/// Removes every voter who isn't a host.
pub async fn remove_all(State(app): State<AppState>, Host(host): Host) -> ApiResult<StatusCode> {
    let removed = host.meeting.lock().await.remove_non_hosts()?;
    for voter in &removed {
        app.end_voter_sessions(host.meeting.id, *voter);
        app.trustauth().drop_voter(host.meeting.id, *voter).await;
    }
    host.meeting.changed();
    info!(meeting = %host.meeting.id, removed = removed.len(), "All non-host voters removed");
    Ok(StatusCode::NO_CONTENT)
}
