use async_trait::async_trait;
use axum::{
    Json,
    extract::{FromRequest, State},
    http::StatusCode,
};

use rustsystem_core::{APIError, APIErrorCode, APIHandler, Method};

use crate::{AppState, tally_encrypt::save_encrypted_tally, vote_auth};

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

        let meetings = state.meetings()?;

        if let Some(meeting) = meetings.lock().await.get(&auth.muuid) {
            if let Some(tally) = meeting.vote_auth.get_last_tally() {
                if let Err(e) = save_encrypted_tally(
                    &auth.muuid,
                    tally,
                    meeting
                        .voters.values().map(|v| v.name.clone())
                        .collect(),
                ) {
                    tracing::error!(
                        "Failed to save encrypted tally for meeting {}: {e}",
                        auth.muuid
                    );
                }
                Ok(Json(tally.clone()))
            } else {
                Err(APIError::from_error_code(APIErrorCode::InvalidState))
            }
        } else {
            Err(APIError::from_error_code(APIErrorCode::MUuidNotFound))
        }
    }
}
