use async_trait::async_trait;
use axum::Json;
use axum::extract::FromRequest;

use axum::{extract::State, http::StatusCode};
use base64::Engine;
use base64::prelude::BASE64_STANDARD;
use qrcode::render::svg;
use qrcode::{EcLevel, QrCode};
use serde::{Deserialize, Serialize};
use tracing::info;

use rustsystem_core::{APIError, APIErrorCode, APIHandler, Method};
use uuid::Uuid;

use crate::admin_auth::AdminCred;
use crate::{API_ENDPOINT, AppState, MUuid, UUuid};

use super::auth::AuthHost;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct QrCodeResponse {
    pub qr_svg: String,
    pub invite_link: String,
}

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

        let meetings_guard = {
            let guard = state.read()?;
            guard.clone().meetings
        };

        if let Some(meeting) = meetings_guard.lock().await.get_mut(&auth.muuid) {
            meeting.invite_auth.set_state(true);
            Ok(())
        } else {
            Err(APIError::from_error_code(APIErrorCode::MUuidNotFound))
        }
    }
}

#[derive(Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NewVoterRequestBody {
    pub voter_name: String,
    pub is_host: bool,
}

#[derive(FromRequest)]
pub struct NewVoterRequest {
    auth: AuthHost,
    state: State<AppState>,
    body: Json<NewVoterRequestBody>,
}

pub struct NewVoter;
#[async_trait]
impl APIHandler for NewVoter {
    type State = AppState;
    type Request = NewVoterRequest;
    type SuccessResponse = Json<QrCodeResponse>;

    const METHOD: Method = Method::Post;
    const PATH: &'static str = "/new-voter";
    const SUCCESS_CODE: StatusCode = StatusCode::CREATED;

    async fn route(request: Self::Request) -> Result<Self::SuccessResponse, APIError> {
        let NewVoterRequest {
            auth,
            state: State(state),
            body:
                Json(NewVoterRequestBody {
                    voter_name,
                    is_host,
                }),
        } = request;

        let new_uuuid = Uuid::new_v4();

        let meetings_guard = {
            let guard = state.read()?;
            guard.clone().meetings
        };

        if let Some(meeting) = meetings_guard.lock().await.get_mut(&auth.muuid) {
            if meeting.locked {
                return Err(APIError::from_error_code(APIErrorCode::InvalidState));
            }

            if meeting.has_voter_with_name(&voter_name) {
                return Err(APIError::from_error_code(APIErrorCode::NameTaken));
            } else {
                meeting.add_voter(voter_name, new_uuuid, is_host);
            }

            let admin_cred = if is_host {
                Some(meeting.admin_auth.new_token())
            } else {
                None
            };
            let (qr_svg, invite_link) = gen_qr_code_with_link(auth.muuid, new_uuuid, admin_cred);

            Ok(Json(QrCodeResponse {
                qr_svg,
                invite_link,
            }))
        } else {
            Err(APIError::from_error_code(APIErrorCode::MUuidNotFound))
        }
    }
}

pub fn gen_qr_code(muuid: MUuid, uuuid: UUuid, admin_cred: Option<AdminCred>) -> String {
    let (qr_svg, _) = gen_qr_code_with_link(muuid, uuuid, admin_cred);
    qr_svg
}

pub fn gen_qr_code_with_link(
    muuid: MUuid,
    uuuid: UUuid,
    admin_cred: Option<AdminCred>,
) -> (String, String) {
    info!("Generating new QR for voter id {uuuid} in meeting {muuid}");
    let mut url = format!("{API_ENDPOINT}/login?muuid={muuid}&uuuid={uuuid}");
    if let Some(admin_cred) = admin_cred {
        url.push_str(&format!(
            "&admin_msg={}&admin_sig={}",
            hex::encode(admin_cred.get_msg()),
            admin_cred.get_sig_str()
        ));
    }
    info!("Creating QR code from {url}");

    let code = QrCode::with_error_correction_level(url.as_bytes(), EcLevel::H)
        .expect(&format!("Creation of QR code was unsuccessful. url: {url}"));
    let qr_svg = code.render::<svg::Color>().min_dimensions(200, 200).build();
    let qr_svg_base64 = format!(
        "data:image/svg+xml;base64,{}",
        BASE64_STANDARD.encode(qr_svg)
    );

    (qr_svg_base64, url)
}
