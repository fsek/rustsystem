use async_trait::async_trait;
use axum::{
    Json,
    extract::{FromRequest, State},
    http::StatusCode,
};

use rustsystem_core::{APIError, APIHandler, Method};

use crate::{AppState, vote_auth};

use super::auth::AuthHost;

#[derive(FromRequest)]
pub struct TallyRequest {
    auth: AuthHost,
    state: State<AppState>,
}

pub struct Tally;
#[async_trait]
impl APIHandler for Tally {
    type State = AppState;
    type Request = TallyRequest;
    type SuccessResponse = Json<vote_auth::Tally>;

    const METHOD: Method = Method::Post;
    const PATH: &'static str = "/tally";
    const SUCCESS_CODE: StatusCode = StatusCode::OK;

    async fn route(request: Self::Request) -> Result<Self::SuccessResponse, APIError> {
        let TallyRequest {
            auth,
            state: State(state),
        } = request;

        let meeting = state.get_meeting(auth.muuid).await?;
        let tally_result = meeting.vote_auth.write().await.finalize_round()?;

        // Unlock the meeting during tally phase to allow invitations between voting sessions.
        // This enables hosts to invite new participants while results are being displayed,
        // before starting the next vote round. The meeting will remain unlocked until
        // a new vote starts (which locks it again).
        meeting.unlock();

        Ok(Json(tally_result))
    }
}
