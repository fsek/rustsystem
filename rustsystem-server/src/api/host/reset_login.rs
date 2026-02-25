use rustsystem_core::{APIError, APIErrorCode, APIHandler, Method};
use async_trait::async_trait;
use axum::{
    Json,
    extract::State,
    http::StatusCode,
};
use serde::{Deserialize, Serialize};

use crate::{AppState, UUuid};

use super::auth::AuthHost;
use super::new_voter::{QrCodeResponse, gen_qr_code_with_link};

#[derive(Deserialize, Serialize)]
pub struct ResetLoginRequest {
    pub user_uuuid: UUuid,
}

pub struct ResetLogin;
#[async_trait]
impl APIHandler for ResetLogin {
    type State = AppState;
    type Request = (AuthHost, State<AppState>, Json<ResetLoginRequest>);
    type SuccessResponse = Json<QrCodeResponse>;

    const METHOD: Method = Method::Post;
    const PATH: &'static str = "/reset-login";
    const SUCCESS_CODE: StatusCode = StatusCode::OK;

    async fn route(request: Self::Request) -> Result<Self::SuccessResponse, APIError> {
        let (auth, State(state), Json(ResetLoginRequest { user_uuuid })) = request;

        let meeting = state.get_meeting(auth.muuid).await?;

        let (new_uuuid, admin_cred) = {
            let mut voters = meeting.voters.write().await;
            let mut user = voters
                .remove(&user_uuuid)
                .ok_or_else(|| APIError::from_error_code(APIErrorCode::UUuidNotFound))?;
            user.logged_in = false;

            // Lock ordering: voters → admin_auth. We hold voters.write() and now acquire
            // admin_auth.write() — this ordering is consistent across the codebase.
            let admin_cred = if user.is_host {
                Some(meeting.admin_auth.write().await.new_token())
            } else {
                None
            };

            let new_uuuid = UUuid::new_v4();
            voters.insert(new_uuuid, user);
            (new_uuuid, admin_cred)
        };

        let (qr_svg, invite_link) = gen_qr_code_with_link(auth.muuid, new_uuuid, admin_cred);
        Ok(Json(QrCodeResponse {
            qr_svg,
            invite_link,
        }))
    }
}
