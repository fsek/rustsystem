//! `GET /api/meeting/events` — live updates (`docs/PROTOCOL.md` §5.6).
//!
//! The stream sends only change counters, `{"version": 12, "round": 3, "agenda": 5}`: once on
//! connect, then on every change. `round` moves only when a round opens, closes or resets, and
//! `agenda` when the agenda or its current point changes, so voters' pages refetch
//! `GET /api/meeting` only then; hosts' pages refetch whenever `version` moves. Because
//! events carry no state, a dropped and reconnected stream can never leave a page showing stale
//! data. The stream ends when the meeting closes.

use std::{convert::Infallible, time::Duration};

use axum::{
    extract::State,
    response::sse::{Event, KeepAlive, Sse},
};
use tokio_stream::{Stream, StreamExt, wrappers::WatchStream};

use rustsystem_core::ApiResult;

use crate::{app::AppState, auth::Member};

pub async fn stream(
    State(app): State<AppState>,
    member: Member,
) -> ApiResult<Sse<impl Stream<Item = Result<Event, Infallible>>>> {
    let slot = app.acquire_sse_slot(&member.meeting)?;
    let versions = WatchStream::new(member.meeting.subscribe());
    // The slot moves into the stream, so it is released exactly when the client disconnects.
    let events = versions
        .take_while(move |_| !slot.meeting().is_closed())
        .map(|versions| Ok(Event::default().json_data(versions).unwrap_or_default()));
    Ok(Sse::new(events).keep_alive(KeepAlive::new().interval(Duration::from_secs(15))))
}
