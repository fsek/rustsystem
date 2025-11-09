use api_core::{APIErrorCode, APIHandler};
use api_derive::APIEndpointError;
use axum::{
    Json,
    extract::{FromRequest, State},
    http::StatusCode,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{AppState, UUuid, api::host::auth::AuthHost};

#[derive(FromRequest)]
pub struct VoterListRequest {
    auth: AuthHost,
    state: State<AppState>,
}

#[derive(APIEndpointError)]
#[api(endpoint(method = "GET", path = "/api/host/voter-list"))]
pub enum VoterListError {
    #[api(code = APIErrorCode::MUuidNotFound, status = 404)]
    MUuidNotFound,
}

pub struct VoterList;
impl APIHandler for VoterList {
    type State = AppState;
    type Request = VoterListRequest;

    const SUCCESS_CODE: StatusCode = StatusCode::OK;
    // List of voters' names and corresponding uuuids.
    type SuccessResponse = Json<Vec<(String, String)>>;
    type ErrorResponse = VoterListError;

    async fn route(
        request: Self::Request,
    ) -> api_core::APIResult<Self::SuccessResponse, Self::ErrorResponse> {
        let VoterListRequest {
            auth: AuthHost { uuuid, muuid },
            state,
        } = request;

        if let Some(meeting) = state.meetings.lock().await.get_mut(&muuid) {
            Ok(Json(
                meeting
                    .voters
                    .iter()
                    .map(|(k, v)| (v.name.clone(), k.to_string()))
                    .collect(),
            ))
        } else {
            Err(VoterListError::MUuidNotFound)
        }
    }
}

#[derive(Deserialize, Serialize)]
pub struct VoterIdRequest {
    pub name: String,
}

#[derive(APIEndpointError)]
#[api(endpoint(method = "GET", path = "/api/host/voter-id"))]
pub enum VoterIdError {
    #[api(code = APIErrorCode::MUuidNotFound, status = 404)]
    MUuidNotFound,
    #[api(code = APIErrorCode::VoterNameNotFound, status = 404)]
    VoterNameNotFound,
}

pub struct VoterId;
impl APIHandler for VoterId {
    type State = AppState;
    type Request = (AuthHost, State<AppState>, Json<VoterIdRequest>);

    const SUCCESS_CODE: StatusCode = StatusCode::OK;

    type SuccessResponse = Json<UUuid>;
    type ErrorResponse = VoterIdError;

    async fn route(
        request: Self::Request,
    ) -> api_core::APIResult<Self::SuccessResponse, Self::ErrorResponse> {
        let (AuthHost { uuuid, muuid }, State(state), Json(VoterIdRequest { name })) = request;

        if let Some(meeting) = state.meetings.lock().await.get_mut(&muuid) {
            if let Some((uuuid, _voter)) = meeting.voters.iter().find(|(_k, v)| v.name == name) {
                Ok(Json(*uuuid))
            } else {
                Err(VoterIdError::VoterNameNotFound)
            }
        } else {
            Err(VoterIdError::MUuidNotFound)
        }
    }
}

#[derive(Deserialize, Serialize)]
pub struct RemoveVoterRequest {
    pub voter_uuuid: UUuid,
}

#[derive(APIEndpointError)]
#[api(endpoint(method = "DELETE", path = "/api/host/remove-voter"))]
pub enum RemoveVoterError {
    #[api(code = APIErrorCode::MUuidNotFound, status = 404)]
    MUuidNotFound,
    #[api(code = APIErrorCode::InvalidUUuid, status = 400)]
    InvalidUUuid,
    #[api(code = APIErrorCode::UUuidNotFound, status = 404)]
    UUuidNotFound,
}

pub struct RemoveVoter;
impl APIHandler for RemoveVoter {
    type State = AppState;
    type Request = (AuthHost, State<AppState>, Json<RemoveVoterRequest>);

    const SUCCESS_CODE: StatusCode = StatusCode::OK;

    type SuccessResponse = ();
    type ErrorResponse = RemoveVoterError;

    async fn route(
        request: Self::Request,
    ) -> api_core::APIResult<Self::SuccessResponse, Self::ErrorResponse> {
        let (AuthHost { uuuid, muuid }, State(state), Json(RemoveVoterRequest { voter_uuuid })) =
            request;

        if let Some(meeting) = state.meetings.lock().await.get_mut(&muuid) {
            meeting
                .voters
                .remove(&voter_uuuid)
                .ok_or(RemoveVoterError::UUuidNotFound)?;
            Ok(())
        } else {
            Err(RemoveVoterError::MUuidNotFound)
        }
    }
}
