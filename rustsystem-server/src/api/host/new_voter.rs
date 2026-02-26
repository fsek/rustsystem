use async_trait::async_trait;
use axum::Json;
use axum::extract::FromRequest;

use axum::{extract::State, http::StatusCode};
use base64::Engine;
use base64::prelude::BASE64_STANDARD;
use qrcode::render::svg;
use qrcode::{EcLevel, QrCode};
use serde::{Deserialize, Serialize};
use std::sync::atomic::Ordering;
use tracing::info;

use rustsystem_core::{APIError, APIErrorCode, APIHandler, Method};
use uuid::Uuid;

use crate::admin_auth::AdminCred;
use crate::{API_ENDPOINT_SERVER, AppState, MUuid, UUuid, Voter};

use super::auth::AuthHost;

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct QrCodeResponse {
    pub qr_svg: String,
    pub invite_link: String,
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
        let meeting = state.get_meeting(auth.muuid).await?;

        if meeting.locked.load(Ordering::Relaxed) {
            return Err(APIError::from_error_code(APIErrorCode::InvalidState));
        }

        let admin_cred = {
            // Acquire voters.write() first, then admin_auth.write() if needed —
            // consistent with the voters → admin_auth lock ordering.
            let mut voters = meeting.voters.write().await;
            if voters.iter().any(|(_, v)| v.name == voter_name) {
                return Err(APIError::from_error_code(APIErrorCode::NameTaken));
            }
            voters.insert(
                new_uuuid,
                Voter {
                    name: voter_name,
                    logged_in: false,
                    is_host,
                    registered_at: std::time::SystemTime::now(),
                },
            );
            if is_host {
                Some(meeting.admin_auth.write().await.new_token())
            } else {
                None
            }
        };

        let (qr_svg, invite_link) = gen_qr_code_with_link(auth.muuid, new_uuuid, admin_cred);
        Ok(Json(QrCodeResponse {
            qr_svg,
            invite_link,
        }))
    }
}

pub fn gen_qr_code_with_link(
    muuid: MUuid,
    uuuid: UUuid,
    admin_cred: Option<AdminCred>,
) -> (String, String) {
    info!("Generating new QR for voter id {uuuid} in meeting {muuid}");
    let mut url = format!("{API_ENDPOINT_SERVER}/login?muuid={muuid}&uuuid={uuuid}");
    if let Some(admin_cred) = admin_cred {
        url.push_str(&format!(
            "&admin_msg={}&admin_sig={}",
            hex::encode(admin_cred.get_msg()),
            admin_cred.get_sig_str()
        ));
    }
    info!("Creating QR code from {url}");

    let code = QrCode::with_error_correction_level(url.as_bytes(), EcLevel::H)
        .unwrap_or_else(|_| panic!("Creation of QR code was unsuccessful. url: {url}"));
    let qr_svg = code.render::<svg::Color>().min_dimensions(200, 200).build();
    let qr_svg_base64 = format!(
        "data:image/svg+xml;base64,{}",
        BASE64_STANDARD.encode(qr_svg)
    );

    (qr_svg_base64, url)
}
