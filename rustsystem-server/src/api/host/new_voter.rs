use axum::http::header;
use axum::{
    extract::State,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use qrcode::render::svg;
use qrcode::{EcLevel, QrCode};
use tracing::info;

use crate::{API_ENDPOINT, AppState, new_uuid};
use crate::{MUID, UUID};

use super::auth::AuthHost;

pub async fn start_invite(
    AuthHost { uuid, muid }: AuthHost,
    State(state): State<AppState>,
) -> Response {
    if let Some(meeting) = state.meetings.lock().await.get_mut(&muid) {
        meeting.invite_auth.set_state(true);
        StatusCode::OK.into_response()
    } else {
        StatusCode::NOT_FOUND.into_response()
    }
}

pub async fn new_voter(
    AuthHost { uuid, muid }: AuthHost,
    State(state): State<AppState>,
) -> Response {
    let new_uuid = new_uuid();
    if let Some(meeting) = state.meetings.lock().await.get_mut(&muid) {
        meeting.invite_auth.set_state(false);
        // This isn't guaranteed but backed by 128 bits of entropy. Should be okay.
        meeting.add_voter(new_uuid);
    } else {
        return (StatusCode::FORBIDDEN).into_response();
    }

    let qr_svg = gen_qr_code(muid, new_uuid);

    (
        StatusCode::CREATED,
        [(header::CONTENT_TYPE, "image/svg+xml")],
        qr_svg,
    )
        .into_response()
}

fn gen_qr_code(muid: MUID, uuid: UUID) -> String {
    info!("Generating new QR for voter id {uuid} in meeting {muid}");
    let url = format!("{API_ENDPOINT}/login?muid=\"{muid}\"&uuid=\"{uuid}\"");
    info!("{url}");

    let code = QrCode::with_error_correction_level(url.as_bytes(), EcLevel::H).unwrap();
    code.render::<svg::Color>().min_dimensions(200, 200).build()
}
