use rustsystem_core::{APIError, APIErrorCode, APIHandler, Method};
use async_trait::async_trait;
use axum::{
    extract::{FromRequest, State},
    http::StatusCode,
};

use crate::AppState;

use super::auth::AuthHost;

#[derive(FromRequest)]
pub struct RemoveAllRequest {
    auth: AuthHost,
    state: State<AppState>,
}

pub struct RemoveAll;
#[async_trait]
impl APIHandler for RemoveAll {
    type State = AppState;
    type Request = RemoveAllRequest;
    type SuccessResponse = ();

    const METHOD: Method = Method::Delete;
    const PATH: &'static str = "/remove-all";
    const SUCCESS_CODE: StatusCode = StatusCode::OK;

    async fn route(request: Self::Request) -> Result<Self::SuccessResponse, APIError> {
        let RemoveAllRequest {
            auth,
            state: State(state),
        } = request;

        let meetings = state.meetings()?;
        if let Some(meeting) = meetings.lock().await.get_mut(&auth.muuid) {
            meeting.voters.retain(|_uuid, voter| voter.is_host);

            Ok(())
        } else {
            Err(APIError::from_error_code(APIErrorCode::MUuidNotFound))
        }
    }
}
