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

        let meetings = state.meetings()?;
        if let Some(meeting) = meetings.lock().await.get_mut(&auth.muuid) {
            if let Some(mut user) = meeting.voters.remove(&user_uuuid) {
                user.logged_in = false;

                let admin_cred = if user.is_host {
                    Some(meeting.admin_auth.new_token())
                } else {
                    None
                };

                let new_uuuid = UUuid::new_v4();
                meeting.voters.insert(new_uuuid, user);

                let (qr_svg, invite_link) =
                    gen_qr_code_with_link(auth.muuid, new_uuuid, admin_cred);
                Ok(Json(QrCodeResponse {
                    qr_svg,
                    invite_link,
                }))
            } else {
                Err(APIError::from_error_code(APIErrorCode::UUuidNotFound))
            }
        } else {
            Err(APIError::from_error_code(APIErrorCode::MUuidNotFound))
        }
    }
}
