use axum::{
    Json,
    extract::{FromRequest, State},
    http::StatusCode,
};
use rustsystem_proof::BallotMetaData;
use serde::{Deserialize, Serialize};
use tracing::info;

use crate::{
    AppState,
    api::APIHandler,
    vote_auth::{self, TallyError},
};

use super::auth::AuthHost;

#[derive(Deserialize)]
pub struct StartVoteRequest {
    name: String,
    metadata: BallotMetaData,
}

#[derive(Serialize)]
pub enum StartVoteError {
    MUIDNotFound,
}

pub struct StartVote;
impl APIHandler for StartVote {
    type State = AppState;
    type Request = (AuthHost, State<AppState>, Json<StartVoteRequest>);
    type SuccessResponse = ();
    type ErrorResponse = Json<StartVoteError>;
    async fn handler(
        request: Self::Request,
    ) -> crate::api::APIResponse<Self::SuccessResponse, Self::ErrorResponse> {
        let (AuthHost { uuid, muid }, State(state), Json(body)) = request;

        if let Some(meeting) = state.meetings.lock().await.get_mut(&muid) {
            info!("Starting vote: {}", body.name);
            meeting.get_auth().start_round(body.metadata, body.name);

            return Ok((StatusCode::OK, ()));
        } else {
            return Err((StatusCode::NOT_FOUND, Json(StartVoteError::MUIDNotFound)));
        }
    }
}

#[derive(FromRequest)]
pub struct TallyRequest {
    auth: AuthHost,
    state: State<AppState>,
}

pub struct Tally;
impl APIHandler for Tally {
    type State = AppState;
    type Request = TallyRequest;
    type SuccessResponse = Json<vote_auth::Tally>;
    type ErrorResponse = Json<TallyError>;

    async fn handler(
        request: Self::Request,
    ) -> crate::api::APIResponse<Self::SuccessResponse, Self::ErrorResponse> {
        let TallyRequest {
            auth: AuthHost { uuid, muid },
            state: State(state),
        } = request;

        if let Some(meeting) = state.meetings.lock().await.get_mut(&muid) {
            let vote_auth = meeting.get_auth();

            Ok((StatusCode::OK, Json(vote_auth.finalize_round()?)))
        } else {
            Err((StatusCode::NOT_FOUND, Json(TallyError::MUIDNotFound)))
        }
    }
}
