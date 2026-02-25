use async_trait::async_trait;
use axum::{
    Json,
    extract::{FromRequest, State},
    http::StatusCode,
};

use rustsystem_core::{APIError, APIErrorCode, APIHandler, Method};

use crate::{AppState, vote_auth};

use super::auth::AuthHost;

#[derive(FromRequest)]
pub struct GetTallyRequest {
    auth: AuthHost,
    state: State<AppState>,
}

pub struct GetTally;
#[async_trait]
impl APIHandler for GetTally {
    type State = AppState;
    type Request = GetTallyRequest;
    type SuccessResponse = Json<vote_auth::Tally>;

    const METHOD: Method = Method::Get;
    const PATH: &'static str = "/get-tally";
    const SUCCESS_CODE: StatusCode = StatusCode::OK;

    async fn route(request: Self::Request) -> Result<Self::SuccessResponse, APIError> {
        let GetTallyRequest {
            auth,
            state: State(state),
        } = request;

        let meeting = state.get_meeting(auth.muuid).await?;

        let tally = {
            let vote_auth = meeting.vote_auth.read().await;
            vote_auth
                .get_last_tally()
                .cloned()
                .ok_or_else(|| APIError::from_error_code(APIErrorCode::InvalidState))?
        };

        Ok(Json(tally))
    }
}
