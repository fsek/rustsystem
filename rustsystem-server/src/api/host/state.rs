use api_derive::APIEndpointError;
use axum::{
    Json,
    extract::{FromRequest, State},
    http::StatusCode,
};
use rustsystem_proof::BallotMetaData;
use serde::{Deserialize, Serialize};
use tracing::info;

use api_core::{APIErrorCode, APIHandler, APIResponse, APIResult};

use crate::{
    AppState,
    vote_auth::{self, TallyError},
};

use super::auth::AuthHost;

#[derive(Deserialize)]
pub struct StartVoteRequest {
    name: String,
    metadata: BallotMetaData,
}

#[derive(APIEndpointError)]
#[api(endpoint(method = "POST", path = "/api/host/start-vote"))]
pub enum StartVoteError {
    #[api(code = APIErrorCode::MUIDNotFound, status = 404)]
    MUIDNotFound,
}

pub struct StartVote;
impl APIHandler for StartVote {
    type State = AppState;
    type Request = (AuthHost, State<AppState>, Json<StartVoteRequest>);

    const SUCCESS_CODE: StatusCode = StatusCode::OK;
    type SuccessResponse = ();
    type ErrorResponse = StartVoteError;
    async fn route(
        request: Self::Request,
    ) -> APIResult<Self::SuccessResponse, Self::ErrorResponse> {
        let (AuthHost { uuid, muid }, State(state), Json(body)) = request;

        if let Some(meeting) = state.meetings.lock().await.get_mut(&muid) {
            info!("Starting vote: {}", body.name);
            meeting.get_auth().start_round(body.metadata, body.name);

            return Ok(());
        } else {
            return Err(StartVoteError::MUIDNotFound);
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

    const SUCCESS_CODE: StatusCode = StatusCode::OK;
    type SuccessResponse = Json<vote_auth::Tally>;
    type ErrorResponse = TallyError;

    async fn route(
        request: Self::Request,
    ) -> APIResult<Self::SuccessResponse, Self::ErrorResponse> {
        let TallyRequest {
            auth: AuthHost { uuid, muid },
            state: State(state),
        } = request;

        if let Some(meeting) = state.meetings.lock().await.get_mut(&muid) {
            let vote_auth = meeting.get_auth();

            Ok(Json(vote_auth.finalize_round()?))
        } else {
            Err(TallyError::MUIDNotFound)
        }
    }
}
