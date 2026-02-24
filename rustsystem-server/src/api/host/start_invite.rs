use async_trait::async_trait;
use axum::extract::FromRequest;

use axum::{extract::State, http::StatusCode};

use rustsystem_core::{APIError, APIErrorCode, APIHandler, Method};

use crate::AppState;

use super::auth::AuthHost;

#[derive(FromRequest)]
pub struct StartInviteRequest {
    auth: AuthHost,
    state: State<AppState>,
}

pub struct StartInvite;
#[async_trait]
impl APIHandler for StartInvite {
    type State = AppState;
    type Request = StartInviteRequest;
    type SuccessResponse = ();

    const METHOD: Method = Method::Post;
    const PATH: &'static str = "/start-invite";
    const SUCCESS_CODE: StatusCode = StatusCode::OK;

    async fn route(request: Self::Request) -> Result<Self::SuccessResponse, APIError> {
        let StartInviteRequest {
            auth,
            state: State(state),
        } = request;

        let meetings = state.meetings()?;

        if let Some(meeting) = meetings.lock().await.get_mut(&auth.muuid) {
            meeting.invite_auth.set_state(true);
            Ok(())
        } else {
            Err(APIError::from_error_code(APIErrorCode::MUuidNotFound))
        }
    }
}
