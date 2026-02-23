use api_core::{APIError, APIErrorCode, APIHandler, Method};
use async_trait::async_trait;
use axum::{
    Json,
    extract::{FromRequest, State},
    http::StatusCode,
};
use serde::{Deserialize, Serialize};

use crate::{
    AppState, UUuid,
    api::host::{
        auth::AuthHost,
        new_voter::{QrCodeResponse, gen_qr_code_with_link},
    },
};

#[derive(Serialize, Deserialize, Debug)]
pub struct VoterInfo {
    pub name: String,
    pub uuid: String,
    pub registered_at: u64,
    pub logged_in: bool,
    pub is_host: bool,
}

#[derive(FromRequest)]
pub struct VoterListRequest {
    auth: AuthHost,
    state: State<AppState>,
}

pub struct VoterList;
#[async_trait]
impl APIHandler for VoterList {
    type State = AppState;
    type Request = VoterListRequest;
    type SuccessResponse = Json<Vec<VoterInfo>>;

    const METHOD: Method = Method::Get;
    const PATH: &'static str = "/voter-list";
    const SUCCESS_CODE: StatusCode = StatusCode::OK;

    async fn route(request: Self::Request) -> Result<Self::SuccessResponse, APIError> {
        let VoterListRequest { auth, state } = request;

        let meetings_guard = {
            let guard = state.read()?;
            guard.clone().meetings
        };
        if let Some(meeting) = meetings_guard.lock().await.get_mut(&auth.muuid) {
            Ok(Json(
                meeting
                    .voters
                    .iter()
                    .map(|(k, v)| VoterInfo {
                        name: v.name.clone(),
                        uuid: k.to_string(),
                        registered_at: v
                            .registered_at
                            .duration_since(std::time::UNIX_EPOCH)
                            .unwrap_or_default()
                            .as_secs(),
                        logged_in: v.logged_in,
                        is_host: v.is_host,
                    })
                    .collect(),
            ))
        } else {
            Err(APIError::from_error_code(APIErrorCode::MUuidNotFound))
        }
    }
}

#[derive(Deserialize, Serialize)]
pub struct VoterIdRequest {
    pub name: String,
}

pub struct VoterId;
#[async_trait]
impl APIHandler for VoterId {
    type State = AppState;
    type Request = (AuthHost, State<AppState>, Json<VoterIdRequest>);
    type SuccessResponse = Json<UUuid>;

    const METHOD: Method = Method::Get;
    const PATH: &'static str = "/voter-id";
    const SUCCESS_CODE: StatusCode = StatusCode::OK;

    async fn route(request: Self::Request) -> Result<Self::SuccessResponse, APIError> {
        let (auth, State(state), Json(VoterIdRequest { name })) = request;

        let meetings_guard = {
            let guard = state.read()?;
            guard.clone().meetings
        };

        if let Some(meeting) = meetings_guard.lock().await.get_mut(&auth.muuid) {
            if let Some((uuuid, _voter)) = meeting.voters.iter().find(|(_k, v)| v.name == name) {
                Ok(Json(*uuuid))
            } else {
                Err(APIError::from_error_code(APIErrorCode::VoterNameNotFound))
            }
        } else {
            Err(APIError::from_error_code(APIErrorCode::MUuidNotFound))
        }
    }
}

#[derive(Deserialize, Serialize)]
pub struct RemoveVoterRequest {
    pub voter_uuuid: UUuid,
}

pub struct RemoveVoter;
#[async_trait]
impl APIHandler for RemoveVoter {
    type State = AppState;
    type Request = (AuthHost, State<AppState>, Json<RemoveVoterRequest>);
    type SuccessResponse = ();

    const METHOD: Method = Method::Delete;
    const PATH: &'static str = "/remove-voter";
    const SUCCESS_CODE: StatusCode = StatusCode::OK;

    async fn route(request: Self::Request) -> Result<Self::SuccessResponse, APIError> {
        let (auth, State(state), Json(RemoveVoterRequest { voter_uuuid })) = request;

        let meetings_guard = {
            let guard = state.read()?;
            guard.clone().meetings
        };
        if let Some(meeting) = meetings_guard.lock().await.get_mut(&auth.muuid) {
            meeting
                .voters
                .remove(&voter_uuuid)
                .ok_or(APIError::from_error_code(APIErrorCode::UUuidNotFound))?;
            Ok(())
        } else {
            Err(APIError::from_error_code(APIErrorCode::MUuidNotFound))
        }
    }
}

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

        let meetings_guard = {
            let guard = state.read()?;
            guard.clone().meetings
        };
        if let Some(meeting) = meetings_guard.lock().await.get_mut(&auth.muuid) {
            meeting.voters.retain(|_uuid, voter| voter.is_host);

            Ok(())
        } else {
            Err(APIError::from_error_code(APIErrorCode::MUuidNotFound))
        }
    }
}

#[derive(Deserialize, Serialize)]
pub struct ResetLoginRequest {
    pub user_uuuid: UUuid,
}

pub struct ResetLogin;
#[async_trait]
impl APIHandler for ResetLogin {
    type State = AppState;
    type Request = (AuthHost, State<AppState>, Json<ResetLoginRequest>);
    type SuccessResponse = Json<QrCodeResponse>;

    const METHOD: Method = Method::Post;
    const PATH: &'static str = "/reset-login";
    const SUCCESS_CODE: StatusCode = StatusCode::OK;

    async fn route(request: Self::Request) -> Result<Self::SuccessResponse, APIError> {
        let (auth, State(state), Json(ResetLoginRequest { user_uuuid })) = request;

        let meetings_guard = {
            let guard = state.read()?;
            guard.clone().meetings
        };
        if let Some(meeting) = meetings_guard.lock().await.get_mut(&auth.muuid) {
            if let Some(mut user) = meeting.voters.remove(&user_uuuid) {
                user.logged_in = false;

                let admin_cred = if user.is_host {
                    Some(meeting.admin_auth.new_token())
                } else {
                    None
                };

                let new_uuuid = UUuid::new_v4();
                meeting.voters.insert(new_uuuid, user);

                let (qr_svg, invite_link) =
                    gen_qr_code_with_link(auth.muuid, new_uuuid, admin_cred);
                Ok(Json(QrCodeResponse {
                    qr_svg,
                    invite_link,
                }))
            } else {
                Err(APIError::from_error_code(APIErrorCode::UUuidNotFound))
            }
        } else {
            Err(APIError::from_error_code(APIErrorCode::MUuidNotFound))
        }
    }
}
