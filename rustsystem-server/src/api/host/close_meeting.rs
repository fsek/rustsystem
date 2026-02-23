use api_core::{APIError, APIErrorCode, APIHandler, Method};
use async_trait::async_trait;
use axum::{
    extract::{FromRequest, State},
    http::StatusCode,
};

use crate::{AppState, api::host::auth::AuthHost};

#[derive(FromRequest)]
pub struct CloseMeetingRequest {
    auth: AuthHost,
    state: State<AppState>,
}

pub struct CloseMeeting;
#[async_trait]
impl APIHandler for CloseMeeting {
    type State = AppState;
    type Request = CloseMeetingRequest;

    const METHOD: Method = Method::Delete;
    const PATH: &'static str = "/close-meeting";
    const SUCCESS_CODE: axum::http::StatusCode = StatusCode::OK;
    type SuccessResponse = ();

    async fn route(request: Self::Request) -> Result<Self::SuccessResponse, APIError> {
        let CloseMeetingRequest { auth, state } = request;

        let meetings_guard = {
            let guard = state.read()?;
            guard.clone().meetings
        };

        meetings_guard
            .lock()
            .await
            .remove(&auth.muuid)
            .ok_or(APIError::from_error_code(APIErrorCode::MUuidNotFound))?;
        Ok(())
    }
}
