use api_derive::APIEndpointError;
use axum::extract::FromRequest;
use axum::http::header;
use axum::{extract::State, http::StatusCode};
use qrcode::render::svg;
use qrcode::{EcLevel, QrCode};
use tracing::info;

use api_core::{APIErrorCode, APIHandler, APIResult};

use crate::{API_ENDPOINT, AppState, MUID, UUID, new_uuid};

use super::auth::AuthHost;

#[derive(FromRequest)]
pub struct StartInviteRequest {
    auth: AuthHost,
    state: State<AppState>,
}

#[derive(APIEndpointError)]
#[api(endpoint(method = "POST", path = "/api/host/start-invite"))]
pub enum StartInviteError {
    #[api(code = APIErrorCode::MUIDNotFound status = 404)]
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
            auth: AuthHost { uuid, muid },
            state: State(state),
        } = request;

        if let Some(meeting) = state.meetings.lock().await.get_mut(&muid) {
            meeting.invite_auth.set_state(true);
            Ok(())
        } else {
            Err(StartInviteError::MUIDNotFound)
        }
    }
}

#[derive(FromRequest)]
pub struct NewVoterRequest {
    auth: AuthHost,
    state: State<AppState>,
}

#[derive(APIEndpointError)]
#[api(endpoint(method = "POST", path = "/api/host/new-voter"))]
pub enum NewVoterError {
    #[api(code = APIErrorCode::MUIDNotFound, status = 404)]
    MUIDNotFound,
    #[api(code = APIErrorCode::MeetingLocked, status = 409)]
    MeetingLocked,
}

pub struct NewVoter;
impl APIHandler for NewVoter {
    type State = AppState;
    type Request = NewVoterRequest;

    const SUCCESS_CODE: StatusCode = StatusCode::CREATED;
    type SuccessResponse = ([(header::HeaderName, &'static str); 1], String);
    type ErrorResponse = NewVoterError;
    async fn route(
        request: Self::Request,
    ) -> APIResult<Self::SuccessResponse, Self::ErrorResponse> {
        let NewVoterRequest {
            auth: AuthHost { uuid, muid },
            state: State(state),
        } = request;

        let new_uuid = new_uuid();
        if let Some(meeting) = state.meetings.lock().await.get_mut(&muid) {
            if meeting.locked {
                return Err(NewVoterError::MeetingLocked);
            }

            meeting.invite_auth.set_state(false);
            // This isn't guaranteed but backed by 128 bits of entropy. Should be okay.
            meeting.add_voter(new_uuid);
        } else {
            return Err(NewVoterError::MUIDNotFound);
        }

        let qr_svg = gen_qr_code(muid, new_uuid);

        Ok(([(header::CONTENT_TYPE, "image/svg+xml")], qr_svg))
    }
}

fn gen_qr_code(muid: MUID, uuid: UUID) -> String {
    info!("Generating new QR for voter id {uuid} in meeting {muid}");
    let url = format!("{API_ENDPOINT}/login?muid=\"{muid}\"&uuid=\"{uuid}\"");
    info!("{url}");

    let code = QrCode::with_error_correction_level(url.as_bytes(), EcLevel::H).unwrap();
    code.render::<svg::Color>().min_dimensions(200, 200).build()
}
