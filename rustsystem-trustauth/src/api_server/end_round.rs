use rustsystem_core::{APIError, APIErrorCode, APIHandler, Method};
use async_trait::async_trait;
use axum::{Json, extract::State, http::StatusCode};
use serde::Deserialize;
use tracing::info;
use uuid::Uuid;

use crate::AppState;

#[derive(Deserialize)]
pub struct EndRoundRequest {
    pub muuid: Uuid,
}

pub struct EndRound;

#[async_trait]
impl APIHandler for EndRound {
    type State = AppState;
    type Request = (State<AppState>, Json<EndRoundRequest>);
    type SuccessResponse = ();

    const METHOD: Method = Method::Post;
    const PATH: &'static str = "/end-round";
    const SUCCESS_CODE: StatusCode = StatusCode::OK;

    async fn route(request: Self::Request) -> Result<Self::SuccessResponse, APIError> {
        let (State(state), Json(body)) = request;

        let removed = state
            .rounds_write()
            .write()
            .await
            .remove(&body.muuid)
            .is_some();

        if removed {
            info!(muuid = %body.muuid, "Vote round removed from trustauth");
        } else {
            return Err(APIError::from_error_code(APIErrorCode::MUuidNotFound));
        }

        Ok(())
    }
}
