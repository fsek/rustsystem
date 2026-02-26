use rustsystem_core::{APIError, APIHandler, Method};
use async_trait::async_trait;
use axum::{
    extract::{FromRequest, State},
    http::StatusCode,
};
use tracing::info;

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
        let mut voters = meeting.voters.write().await;
        let before = voters.len();
        voters.retain(|_uuid, voter| voter.is_host);
        let after = voters.len();

        info!(
            muuid = %auth.muuid,
            removed = before - after,
            hosts_retained = after,
            "All non-host voters removed"
        );

        Ok(())
    }
}
