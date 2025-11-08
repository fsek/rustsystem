use api_core::{APIErrorCode, APIHandler};
use api_derive::APIEndpointError;
use axum::{
    Json,
    extract::{FromRequest, State},
    http::StatusCode,
};
use uuid::Uuid;

use crate::{AppState, api::host::auth::AuthHost};

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
            return Err(VoterListError::MUuidNotFound);
        }
    }
}

#[derive(FromRequest)]
pub struct VoterIdRequest {
    auth: AuthHost,
    state: State<AppState>,
    name: String,
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
    type Request = VoterIdRequest;

    const SUCCESS_CODE: StatusCode = StatusCode::OK;

    type SuccessResponse = String;
    type ErrorResponse = VoterIdError;

    async fn route(
        request: Self::Request,
    ) -> api_core::APIResult<Self::SuccessResponse, Self::ErrorResponse> {
        let VoterIdRequest {
            auth: AuthHost { uuuid, muuid },
            state,
            name,
        } = request;

        if let Some(meeting) = state.meetings.lock().await.get_mut(&muuid) {
            if let Some((uuuid, _voter)) = meeting.voters.iter().find(|(_k, v)| v.name == name) {
                Ok(uuuid.to_string())
            } else {
                Err(VoterIdError::VoterNameNotFound)
            }
        } else {
            Err(VoterIdError::MUuidNotFound)
        }
    }
}

#[derive(FromRequest)]
pub struct RemoveVoterRequest {
    auth: AuthHost,
    state: State<AppState>,
    voter_uuuid: String,
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
    type Request = RemoveVoterRequest;

    const SUCCESS_CODE: StatusCode = StatusCode::OK;

    type SuccessResponse = ();
    type ErrorResponse = RemoveVoterError;

    async fn route(
        request: Self::Request,
    ) -> api_core::APIResult<Self::SuccessResponse, Self::ErrorResponse> {
        let RemoveVoterRequest {
            auth: AuthHost { uuuid, muuid },
            state,
            voter_uuuid,
        } = request;

        if let Some(meeting) = state.meetings.lock().await.get_mut(&muuid) {
            if let Ok(voter_uuuid) = Uuid::parse_str(&voter_uuuid) {
                meeting
                    .voters
                    .remove(&voter_uuuid)
                    .ok_or(RemoveVoterError::UUuidNotFound)?;
                Ok(())
            } else {
                Err(RemoveVoterError::InvalidUUuid)
            }
        } else {
            Err(RemoveVoterError::MUuidNotFound)
        }
    }
}
