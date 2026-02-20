use api_core::{APIErrorCode, APIHandler};
use api_derive::APIEndpointError;
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

#[derive(APIEndpointError)]
#[api(endpoint(method = "DELETE", path = "/api/host/close-meeting"))]
pub enum CloseMeetingError {
    #[api(code = APIErrorCode::MUuidNotFound status = 404)]
    MUIDNotFound,
}

pub struct CloseMeeting;
impl APIHandler for CloseMeeting {
    type State = AppState;
    type Request = CloseMeetingRequest;

    const SUCCESS_CODE: axum::http::StatusCode = StatusCode::OK;
    type SuccessResponse = ();
    type ErrorResponse = CloseMeetingError;

    async fn route(
        request: Self::Request,
    ) -> api_core::APIResult<Self::SuccessResponse, Self::ErrorResponse> {
        let CloseMeetingRequest { auth, state } = request;

        state
            .meetings
            .lock()
            .await
            .remove(&auth.muuid)
            .ok_or(CloseMeetingError::MUIDNotFound)?;
        Ok(())
    }
}
