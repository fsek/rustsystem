use rustsystem_core::{APIError, APIErrorCode, APIHandler, Method};
use async_trait::async_trait;
use axum::{
    extract::{FromRequest, State},
    http::StatusCode,
};
use tracing::info;

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

        let title = {
            let meetings = state.meetings_read()?;
            let map = meetings.read().await;
            map.get(&auth.muuid).map(|m| m.title.clone()).unwrap_or_default()
        };

        // Outer map write: removing a meeting entry.
        state
            .meetings_write()?
            .write()
            .await
            .remove(&auth.muuid)
            .ok_or_else(|| APIError::from_error_code(APIErrorCode::MUuidNotFound))?;

        info!(
            muuid = %auth.muuid,
            title = %title,
            "Meeting closed"
        );

        Ok(())
    }
}
