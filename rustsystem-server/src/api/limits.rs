use async_trait::async_trait;
use axum::{Json, extract::{FromRequest, State}, http::StatusCode};
use serde::Serialize;

use rustsystem_core::{APIError, APIHandler, MAX_LABEL_LENGTH, MAX_NAME_LENGTH, Method};

use crate::AppState;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LimitsResponse {
    pub max_name_length: usize,
    pub max_label_length: usize,
}

#[derive(FromRequest)]
pub struct LimitsRequest {
    _state: State<AppState>,
}

pub struct Limits;
#[async_trait]
impl APIHandler for Limits {
    type State = AppState;
    type Request = LimitsRequest;
    type SuccessResponse = Json<LimitsResponse>;

    const METHOD: Method = Method::Get;
    const PATH: &'static str = "/limits";
    const SUCCESS_CODE: StatusCode = StatusCode::OK;

    async fn route(_request: LimitsRequest) -> Result<Self::SuccessResponse, APIError> {
        Ok(Json(LimitsResponse {
            max_name_length: MAX_NAME_LENGTH,
            max_label_length: MAX_LABEL_LENGTH,
        }))
    }
}
