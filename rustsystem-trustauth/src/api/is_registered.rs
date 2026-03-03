use async_trait::async_trait;
use axum::{
    Json,
    extract::{FromRequest, State},
    http::StatusCode,
};
use rustsystem_core::{APIError, APIHandler, Method};
use serde::Serialize;

use crate::{AppState, tokens::TrustAuthUser};

#[derive(FromRequest)]
pub struct IsRegisteredRequest {
    auth: TrustAuthUser,
    state: State<AppState>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct IsRegisteredResponse {
    is_registered: bool,
}

pub struct IsRegistered;
#[async_trait]
impl APIHandler for IsRegistered {
    type State = AppState;
    type Request = IsRegisteredRequest;
    type SuccessResponse = Json<IsRegisteredResponse>;

    const METHOD: Method = Method::Get;
    const PATH: &'static str = "/is-registered";
    const SUCCESS_CODE: StatusCode = StatusCode::OK;

    async fn route(request: Self::Request) -> Result<Self::SuccessResponse, APIError> {
        let IsRegisteredRequest { auth, state } = request;

        let round = state.get_round(auth.muuid).await?;
        let is_registered = round
            .registered_voters
            .read()
            .await
            .contains_key(&auth.uuuid);

        Ok(Json(IsRegisteredResponse { is_registered }))
    }
}
