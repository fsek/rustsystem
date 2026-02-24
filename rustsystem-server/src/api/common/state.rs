use async_trait::async_trait;
use axum::{
    Json,
    extract::{FromRequest, State},
    http::StatusCode,
    response::{Sse, sse::Event},
};
use serde::Serialize;

use rustsystem_core::{APIError, APIErrorCode, APIHandler, Method};
use tokio_stream::{StreamExt, adapters::FilterMap, wrappers::WatchStream};

use crate::{AppState, proof::BallotMetaData, tokens::AuthUser, vote_auth::VoteState};

#[derive(FromRequest)]
pub struct VoteActiveRequest {
    auth: AuthUser,
    state: State<AppState>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VoteActiveResponse {
    is_active: bool,
}

pub struct VoteActive;
#[async_trait]
impl APIHandler for VoteActive {
    type State = AppState;
    type Request = VoteActiveRequest;

    const METHOD: Method = Method::Get;
    const PATH: &'static str = "/vote-active";
    const SUCCESS_CODE: StatusCode = StatusCode::OK;
    type SuccessResponse = Json<VoteActiveResponse>;

    async fn route(request: Self::Request) -> Result<Self::SuccessResponse, APIError> {
        let VoteActiveRequest {
            auth,
            state: State(state),
        } = request;

        let meetings_guard = {
            let guard = state.read()?;
            guard.meetings.clone()
        };

        let res = if let Some(meeting) = meetings_guard.lock().await.get(&auth.muuid) {
            meeting.vote_auth.is_active()
        } else {
            return Err(APIError::from_error_code(APIErrorCode::MUuidNotFound));
        };

        Ok(Json(VoteActiveResponse { is_active: res }))
    }
}

#[derive(FromRequest)]
pub struct VoteStateWatchRequest {
    auth: AuthUser,
    state: State<AppState>,
}

/// Endpoint for waiting for voting round to begin.
///
/// Returns 200 OK along with a server-side-event upon success
pub struct VoteStateWatch;
#[async_trait]
impl APIHandler for VoteStateWatch {
    type State = AppState;
    type Request = VoteStateWatchRequest;

    const METHOD: Method = Method::Get;
    const PATH: &'static str = "/vote-state-watch";
    const SUCCESS_CODE: StatusCode = StatusCode::OK;
    type SuccessResponse =
        Sse<FilterMap<WatchStream<VoteState>, fn(VoteState) -> Option<Result<Event, APIError>>>>;

    async fn route(request: Self::Request) -> Result<Self::SuccessResponse, APIError> {
        let VoteStateWatchRequest {
            auth,
            state: State(state),
        } = request;

        let upon_event = |new_state: VoteState| match new_state {
            VoteState::Creation => Some(Ok::<Event, APIError>(Event::default().data("Creation"))),
            VoteState::Voting => Some(Ok::<Event, APIError>(Event::default().data("Voting"))),
            VoteState::Tally => Some(Ok::<Event, APIError>(Event::default().data("Tally"))),
        };

        let meetings_guard = {
            let guard = state.read()?;
            guard.meetings.clone()
        };

        if let Some(meeting) = meetings_guard.lock().await.get(&auth.muuid) {
            let state_rx = meeting.vote_auth.new_state_watcher();
            let stream = WatchStream::new(state_rx).filter_map(upon_event as _);
            Ok(Sse::new(stream))
        } else {
            Err(APIError::from_error_code(APIErrorCode::MUuidNotFound))
        }
    }
}

#[derive(FromRequest)]
pub struct VoteProgressRequest {
    auth: AuthUser,
    state: State<AppState>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VoteProgressResponse {
    is_active: bool,
    is_tally: bool,
    total_votes_cast: usize,
    total_participants: usize,
    vote_name: Option<String>,
    metadata: Option<BallotMetaData>,
}

pub struct VoteProgress;
#[async_trait]
impl APIHandler for VoteProgress {
    type State = AppState;
    type Request = VoteProgressRequest;

    const METHOD: Method = Method::Get;
    const PATH: &'static str = "/vote-progress";
    const SUCCESS_CODE: StatusCode = StatusCode::OK;
    type SuccessResponse = Json<VoteProgressResponse>;
    async fn route(request: Self::Request) -> Result<Self::SuccessResponse, APIError> {
        let VoteProgressRequest {
            auth,
            state: State(state),
        } = request;

        let meetings_guard = {
            let guard = state.read()?;
            guard.meetings.clone()
        };

        if let Some(meeting) = meetings_guard.lock().await.get(&auth.muuid) {
            let is_active = meeting.vote_auth.is_active();
            let is_tally = meeting.vote_auth.is_tally();

            let total_participants = meeting.voters.len();
            let (total_votes_cast, vote_name, metadata) = if is_active || is_tally {
                if let Some(round) = meeting.vote_auth.round_ref() {
                    let votes_cast = round.get_vote_count();
                    let name = meeting.vote_auth.get_current_vote_name().map(|s| s.clone());
                    let meta = if is_active { Some(round.metadata()) } else { None };
                    (votes_cast, name, meta)
                } else {
                    (0, None, None)
                }
            } else {
                (0, None, None)
            };

            Ok(Json(VoteProgressResponse {
                is_active,
                is_tally,
                total_votes_cast,
                total_participants,
                vote_name,
                metadata,
            }))
        } else {
            Err(APIError::from_error_code(APIErrorCode::MUuidNotFound))
        }
    }
}

#[derive(FromRequest)]
pub struct VoteProgressWatchRequest {
    auth: AuthUser,
    state: State<AppState>,
}

/// Endpoint for watching vote progress updates in real-time
pub struct VoteProgressWatch;
#[async_trait]
impl APIHandler for VoteProgressWatch {
    type State = AppState;
    type Request = VoteProgressWatchRequest;

    const METHOD: Method = Method::Get;
    const PATH: &'static str = "/vote-progress-watch";
    const SUCCESS_CODE: StatusCode = StatusCode::OK;
    type SuccessResponse =
        Sse<FilterMap<WatchStream<bool>, fn(bool) -> Option<Result<Event, APIError>>>>;

    async fn route(request: Self::Request) -> Result<Self::SuccessResponse, APIError> {
        let VoteProgressWatchRequest {
            auth,
            state: State(state),
        } = request;

        let upon_event = |_update: bool| {
            Some(Ok::<Event, APIError>(
                Event::default().data("VoteProgressUpdated"),
            ))
        };

        let meetings_guard = {
            let guard = state.read()?;
            guard.meetings.clone()
        };

        if let Some(meeting) = meetings_guard.lock().await.get(&auth.muuid) {
            let update_rx = meeting.vote_auth.new_update_watcher();
            let stream = WatchStream::new(update_rx).filter_map(upon_event as _);
            Ok(Sse::new(stream))
        } else {
            Err(APIError::from_error_code(APIErrorCode::MUuidNotFound))
        }
    }
}
