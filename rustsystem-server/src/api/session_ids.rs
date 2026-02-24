use async_trait::async_trait;
use axum::{
    Json,
    extract::{FromRequest, State},
    http::StatusCode,
};
use serde::Serialize;

use rustsystem_core::{APIError, APIHandler, Method};

use crate::{AppState, AuthUser};

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

/// Endpoint for checking if the current user is authenticated for a given meeting
///
/// Returns 200 OK upon success
pub struct SessionIds;
#[async_trait]
impl APIHandler for SessionIds {
    type State = AppState;
    type Request = SessionIdsRequest;
    type SuccessResponse = Json<SessionIdsResponse>;

    const METHOD: Method = Method::Get;
    const PATH: &'static str = "/session-ids";
    const SUCCESS_CODE: StatusCode = StatusCode::OK;

    async fn route(request: Self::Request) -> Result<Self::SuccessResponse, APIError> {
        let SessionIdsRequest { auth, _state } = request;
        Ok(Json(SessionIdsResponse {
            uuuid: auth.uuuid.to_string(),
            muuid: auth.muuid.to_string(),
        }))
    }
}
