use rustsystem_core::{APIError, APIHandler, Method};
use async_trait::async_trait;
use axum::{
    extract::{FromRequest, State},
    http::StatusCode,
};

use crate::AppState;

use super::auth::AuthHost;

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

        let meeting = state.get_meeting(auth.muuid).await?;
        meeting
            .voters
            .write()
            .await
            .retain(|_uuid, voter| voter.is_host);

        Ok(())
    }
}
