use api_derive::APIEndpointError;
use axum::{
    Json,
    extract::{FromRequest, State},
    http::StatusCode,
};
use rustsystem_proof::BallotMetaData;
use serde::{Deserialize, Serialize};

use api_core::{APIErrorCode, APIHandler, APIResult};

use crate::{
    AppState,
    vote_auth::{self, TallyError},
};

use super::auth::AuthHost;

#[derive(Deserialize, Serialize)]
pub struct StartVoteRequest {
    pub name: String,
    pub shuffle: bool,
    pub metadata: BallotMetaData,
}

#[derive(APIEndpointError)]
#[api(endpoint(method = "POST", path = "/api/host/start-vote"))]
pub enum StartVoteError {
    #[api(code = APIErrorCode::MUuidNotFound, status = 404)]
    MUIDNotFound,
    #[api(code = APIErrorCode::InvalidState, status = 409)]
    InvalidState,
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
        let (auth, State(state), Json(body)) = request;

        if let Some(meeting) = state.meetings.lock().await.get_mut(&auth.muuid) {
            if meeting.get_auth().is_inactive() {
                meeting.lock();
                meeting
                    .get_auth()
                    .start_round(body.metadata, body.shuffle, body.name);
            } else {
                return Err(StartVoteError::InvalidState);
            }

            Ok(())
        } else {
            Err(StartVoteError::MUIDNotFound)
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
            auth,
            state: State(state),
        } = request;

        if let Some(meeting) = state.meetings.lock().await.get_mut(&auth.muuid) {
            let vote_auth = meeting.get_auth();

            Ok(Json(vote_auth.finalize_round()?))
        } else {
            Err(TallyError::MUIDNotFound)
        }
    }
}

#[derive(FromRequest)]
pub struct EndVoteRoundRequest {
    auth: AuthHost,
    state: State<AppState>,
}

#[derive(APIEndpointError)]
#[api(endpoint(method = "DELETE", path = "/api/host/end-vote-round"))]
pub enum EndVoteRoundError {
    #[api(code = APIErrorCode::MUuidNotFound, status = 404)]
    MUIDNotFound,
}

pub struct EndVoteRound;
impl APIHandler for EndVoteRound {
    type State = AppState;
    type Request = EndVoteRoundRequest;

    const SUCCESS_CODE: StatusCode = StatusCode::OK;
    type SuccessResponse = ();
    type ErrorResponse = EndVoteRoundError;

    async fn route(
        request: Self::Request,
    ) -> APIResult<Self::SuccessResponse, Self::ErrorResponse> {
        let EndVoteRoundRequest {
            auth,
            state: State(state),
        } = request;

        if let Some(meeting) = state.meetings.lock().await.get_mut(&auth.muuid) {
            meeting.get_auth().reset();
            meeting.unlock();
            Ok(())
        } else {
            Err(EndVoteRoundError::MUIDNotFound)
        }
    }
}
