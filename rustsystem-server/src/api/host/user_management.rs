use api_core::{APIErrorCode, APIHandler};
use api_derive::APIEndpointError;
use axum::{
    Json,
    extract::{FromRequest, State},
    http::{StatusCode, header},
};
use serde::{Deserialize, Serialize};

use crate::{
    AppState, UUuid,
    api::host::{auth::AuthHost, new_voter::gen_qr_code},
};

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
        let VoterListRequest { auth, state } = request;

        if let Some(meeting) = state.meetings.lock().await.get_mut(&auth.muuid) {
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
        let (auth, State(state), Json(VoterIdRequest { name })) = request;

        if let Some(meeting) = state.meetings.lock().await.get_mut(&auth.muuid) {
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
        let (auth, State(state), Json(RemoveVoterRequest { voter_uuuid })) = request;

        if let Some(meeting) = state.meetings.lock().await.get_mut(&auth.muuid) {
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

#[derive(Deserialize, Serialize)]
pub struct ResetLoginRequest {
    pub user_uuuid: UUuid,
}

#[derive(APIEndpointError)]
#[api(endpoint(method = "POST", path = "/api/host/reset-login"))]
pub enum ResetLoginError {
    #[api(code = APIErrorCode::MUuidNotFound, status = 404)]
    MUuidNotFound,
    #[api(code = APIErrorCode::InvalidUUuid, status = 400)]
    InvalidUUuid,
    #[api(code = APIErrorCode::UUuidNotFound, status = 404)]
    UUuidNotFound,
}

pub struct ResetLogin;
impl APIHandler for ResetLogin {
    type State = AppState;
    type Request = (AuthHost, State<AppState>, Json<ResetLoginRequest>);

    const SUCCESS_CODE: StatusCode = StatusCode::OK;

    type SuccessResponse = ([(header::HeaderName, &'static str); 1], String);
    type ErrorResponse = ResetLoginError;

    async fn route(
        request: Self::Request,
    ) -> api_core::APIResult<Self::SuccessResponse, Self::ErrorResponse> {
        let (auth, State(state), Json(ResetLoginRequest { user_uuuid })) = request;

        if let Some(meeting) = state.meetings.lock().await.get_mut(&auth.muuid) {
            if let Some(mut user) = meeting.voters.remove(&user_uuuid) {
                user.logged_in = false;

                let admin_cred = if user.is_host {
                    Some(meeting.admin_auth.new_token())
                } else {
                    None
                };

                let new_uuuid = UUuid::new_v4();
                meeting.voters.insert(new_uuuid, user);

                let qr_svg = gen_qr_code(auth.muuid, new_uuuid, admin_cred);
                Ok(([(header::CONTENT_TYPE, "image/svg+xml")], qr_svg))
            } else {
                Err(ResetLoginError::UUuidNotFound)
            }
        } else {
            Err(ResetLoginError::MUuidNotFound)
        }
    }
}
