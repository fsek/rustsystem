use api_derive::APIEndpointError;
use axum::{
    Json,
    extract::{FromRequest, State},
    http::StatusCode,
};
use serde::{Deserialize, Serialize};

use api_core::{APIErrorCode, APIHandler, APIResult};

use crate::{AppState, AuthUser, UUuid};

#[derive(FromRequest)]
pub struct SessionIdsRequest {
    auth: AuthUser,
    _state: State<AppState>,
}

#[derive(Serialize)]
pub struct SessionIdsResponse {
    uuuid: String,
    muuid: String,
}

#[derive(APIEndpointError)]
#[api(endpoint(method = "GET", path = "/api/session-ids"))]
pub enum AuthMeetingError {}

/// Endpoint for checking if the current user is authenticated for a given meeting
///
/// Returns 200 OK upon success
pub struct SessionIds;
impl APIHandler for SessionIds {
    type State = AppState;
    type Request = SessionIdsRequest;

    const SUCCESS_CODE: StatusCode = StatusCode::OK;
    type SuccessResponse = Json<SessionIdsResponse>;
    type ErrorResponse = AuthMeetingError;

    async fn route(
        request: Self::Request,
    ) -> APIResult<Self::SuccessResponse, Self::ErrorResponse> {
        let SessionIdsRequest { auth, _state } = request;
        Ok(Json(SessionIdsResponse {
            uuuid: auth.uuuid.to_string(),
            muuid: auth.muuid.to_string(),
        }))
    }
}
