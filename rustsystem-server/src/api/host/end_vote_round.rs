use async_trait::async_trait;
use axum::{
    extract::{FromRequest, State},
    http::StatusCode,
};

use rustsystem_core::{APIError, APIErrorCode, APIHandler, Method};

use crate::AppState;

use super::auth::AuthHost;

#[derive(FromRequest)]
pub struct EndVoteRoundRequest {
    auth: AuthHost,
    state: State<AppState>,
}

pub struct EndVoteRound;
#[async_trait]
impl APIHandler for EndVoteRound {
    type State = AppState;
    type Request = EndVoteRoundRequest;
    type SuccessResponse = ();

    const METHOD: Method = Method::Delete;
    const PATH: &'static str = "/end-vote-round";
    const SUCCESS_CODE: StatusCode = StatusCode::OK;

    async fn route(request: Self::Request) -> Result<Self::SuccessResponse, APIError> {
        let EndVoteRoundRequest {
            auth,
            state: State(state),
        } = request;

        let meetings = state.meetings()?;
        if let Some(meeting) = meetings.lock().await.get_mut(&auth.muuid) {
            meeting.get_auth().reset();
            // Upon a hard reset (i.e. cancelling the voting round), we unlock
            meeting.unlock();
            Ok(())
        } else {
            Err(APIError::from_error_code(APIErrorCode::MUuidNotFound))
        }
    }
}
