use api_derive::APIEndpointError;
use axum::Json;
use axum::extract::FromRequest;

use axum::{extract::State, http::StatusCode};
use base64::Engine;
use base64::prelude::BASE64_STANDARD;
use qrcode::render::svg;
use qrcode::{EcLevel, QrCode};
use serde::{Deserialize, Serialize};
use tracing::info;

use api_core::{APIErrorCode, APIHandler, APIResult};
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

#[derive(APIEndpointError)]
#[api(endpoint(method = "POST", path = "/api/host/start-invite"))]
pub enum StartInviteError {
    #[api(code = APIErrorCode::MUuidNotFound status = 404)]
    MUIDNotFound,
}

pub struct StartInvite;
impl APIHandler for StartInvite {
    type State = AppState;
    type Request = StartInviteRequest;

    const SUCCESS_CODE: StatusCode = StatusCode::OK;
    type SuccessResponse = ();
    type ErrorResponse = StartInviteError;

    async fn route(
        request: Self::Request,
    ) -> APIResult<Self::SuccessResponse, Self::ErrorResponse> {
        let StartInviteRequest {
            auth,
            state: State(state),
        } = request;

        if let Some(meeting) = state.meetings.lock().await.get_mut(&auth.muuid) {
            meeting.invite_auth.set_state(true);
            Ok(())
        } else {
            Err(StartInviteError::MUIDNotFound)
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

#[derive(APIEndpointError)]
#[api(endpoint(method = "POST", path = "/api/host/new-voter"))]
pub enum NewVoterError {
    #[api(code = APIErrorCode::MUuidNotFound, status = 404)]
    MUIDNotFound,
    #[api(code = APIErrorCode::NameTaken, status = 409)]
    NameTaken,
    #[api(code = APIErrorCode::InvalidState, status = 409)]
    InvalidState,
}

pub struct NewVoter;
impl APIHandler for NewVoter {
    type State = AppState;
    type Request = NewVoterRequest;

    const SUCCESS_CODE: StatusCode = StatusCode::CREATED;
    type SuccessResponse = Json<QrCodeResponse>;
    type ErrorResponse = NewVoterError;
    async fn route(
        request: Self::Request,
    ) -> APIResult<Self::SuccessResponse, Self::ErrorResponse> {
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
        if let Some(meeting) = state.meetings.lock().await.get_mut(&auth.muuid) {
            if meeting.locked {
                return Err(NewVoterError::InvalidState);
            }

            if meeting.has_voter_with_name(&voter_name) {
                return Err(NewVoterError::NameTaken);
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
            Err(NewVoterError::MUIDNotFound)
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
