use async_trait::async_trait;
use axum::{
    Json,
    extract::{FromRequest, State},
    http::StatusCode,
};
use serde::Serialize;

use rustsystem_core::{APIError, APIErrorCode, APIHandler, Method};

use crate::{AppState, proof::BallotMetaData, tokens::AuthUser};

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

        let meetings = state.meetings()?;

        if let Some(meeting) = meetings.lock().await.get(&auth.muuid) {
            let is_active = meeting.vote_auth.is_active();
            let is_tally = meeting.vote_auth.is_tally();

            let total_participants = meeting.voters.len();
            let (total_votes_cast, vote_name, metadata) = if is_active || is_tally {
                if let Some(round) = meeting.vote_auth.round_ref() {
                    let votes_cast = round.get_vote_count();
                    let name = meeting.vote_auth.get_current_vote_name().cloned();
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
