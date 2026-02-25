use async_trait::async_trait;
use axum::{
    Json,
    extract::{FromRequest, State},
    http::StatusCode,
};
use serde::Serialize;

use rustsystem_core::{APIError, APIHandler, Method};

use crate::{AppState, tokens::AuthUser};

#[derive(FromRequest)]
pub struct MeetingSpecsRequest {
    auth: AuthUser,
    state: State<AppState>,
}

#[derive(Serialize)]
pub struct MeetingSpecsResponse {
    title: String,
    participants: usize,
}

pub struct MeetingSpecs;
#[async_trait]
impl APIHandler for MeetingSpecs {
    type State = AppState;
    type Request = MeetingSpecsRequest;
    type SuccessResponse = Json<MeetingSpecsResponse>;

    const METHOD: Method = Method::Get;
    const PATH: &'static str = "/meeting-specs";
    const SUCCESS_CODE: StatusCode = StatusCode::OK;

    async fn route(request: Self::Request) -> Result<Self::SuccessResponse, APIError> {
        let MeetingSpecsRequest {
            auth,
            state: State(state),
        } = request;

        let meeting = state.get_meeting(auth.muuid).await?;
        // title is immutable after construction — no lock needed.
        let title = meeting.title.clone();
        let participants = meeting
            .voters
            .read()
            .await
            .values()
            .filter(|v| v.logged_in)
            .count();

        Ok(Json(MeetingSpecsResponse {
            title,
            participants,
        }))
    }
}
