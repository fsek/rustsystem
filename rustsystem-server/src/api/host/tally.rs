use async_trait::async_trait;
use axum::{
    Json,
    extract::{FromRequest, State},
    http::StatusCode,
};
use tracing::{error, info};

use rustsystem_core::{APIError, APIHandler, Method};

use crate::{AppState, tally_encrypt::save_encrypted_tally, vote_auth};

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

        let round_name = meeting
            .vote_auth
            .read()
            .await
            .get_current_vote_name()
            .cloned()
            .unwrap_or_default();

        let tally_result = meeting.vote_auth.write().await.finalize_round()?;

        // vote_auth read guard released; now safe to read voters independently.
        let voter_names: Vec<String> = meeting
            .voters
            .read()
            .await
            .values()
            .map(|v| v.name.clone())
            .collect();

        let total_votes = tally_result.score.values().sum::<usize>() + tally_result.blank;

        if let Err(e) = save_encrypted_tally(&auth.muuid, &tally_result, voter_names) {
            error!(
                muuid = %auth.muuid,
                round = %round_name,
                "Failed to save encrypted tally: {e}"
            );
        }

        let score_summary: Vec<String> = {
            let mut pairs: Vec<_> = tally_result.score.iter().collect();
            pairs.sort_by_key(|(k, _)| k.as_str());
            pairs.iter().map(|(k, v)| format!("{k}:{v}")).collect()
        };

        info!(
            muuid = %auth.muuid,
            round = %round_name,
            total_votes = total_votes,
            blank_votes = tally_result.blank,
            scores = %score_summary.join(", "),
            "Vote round tallied"
        );

        Ok(Json(tally_result))
    }
}
