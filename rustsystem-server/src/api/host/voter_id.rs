use rustsystem_core::{APIError, APIErrorCode, APIHandler, Method};
use async_trait::async_trait;
use axum::{
    Json,
    extract::State,
    http::StatusCode,
};
use serde::{Deserialize, Serialize};

use crate::{AppState, UUuid};

use super::auth::AuthHost;

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

        let meeting = state.get_meeting(auth.muuid).await?;
        let voters = meeting.voters.read().await;

        if let Some((uuuid, _)) = voters.iter().find(|(_, v)| v.name == name) {
            Ok(Json(*uuuid))
        } else {
            Err(APIError::from_error_code(APIErrorCode::VoterNameNotFound))
        }
    }
}
