use rustsystem_core::{APIError, APIErrorCode, APIHandler, Method};
use async_trait::async_trait;
use axum::{
    Json,
    extract::{FromRequest, State},
    http::StatusCode,
};
use serde::{Deserialize, Serialize};

use crate::AppState;

use super::auth::AuthHost;

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

        let meetings = state.meetings()?;
        if let Some(meeting) = meetings.lock().await.get_mut(&auth.muuid) {
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
