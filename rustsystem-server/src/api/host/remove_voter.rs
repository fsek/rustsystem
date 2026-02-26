use rustsystem_core::{APIError, APIErrorCode, APIHandler, Method};
use async_trait::async_trait;
use axum::{
    Json,
    extract::State,
    http::StatusCode,
};
use serde::{Deserialize, Serialize};
use tracing::info;

use crate::{AppState, UUuid};

use super::auth::AuthHost;

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

        let meeting = state.get_meeting(auth.muuid).await?;
        let voter = meeting
            .voters
            .write()
            .await
            .remove(&voter_uuuid)
            .ok_or_else(|| APIError::from_error_code(APIErrorCode::UUuidNotFound))?;

        info!(
            muuid = %auth.muuid,
            uuuid = %voter_uuuid,
            voter = %voter.name,
            was_logged_in = voter.logged_in,
            "Voter removed"
        );

        Ok(())
    }
}
