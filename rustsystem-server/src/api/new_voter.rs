use axum::http::header;
use axum::{
    Json,
    extract::State,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use tracing::{error, info};

use qirust::helper::{FrameStyle, generate_frameqr};
use qirust::qrcode::QrCodeEcc;

use crate::{API_ENDPOINT, AppState, new_uuid, tokens::AuthUser};
use crate::{MUID, UUID};

pub async fn new_voter(
    AuthUser {
        uuid,
        muid,
        is_host,
    }: AuthUser,
    State(state): State<AppState>,
) -> Response {
    if !is_host {
        // Only host is allowed to create new users
        return StatusCode::UNAUTHORIZED.into_response();
    }

    let new_uuid = new_uuid();
    if let Some(meeting) = state.meetings.lock().await.get_mut(&muid) {
        // This isn't guaranteed but backed by 128 bits of entropy. Should be okay.
        meeting.add_voter(new_uuid);
    } else {
        return (StatusCode::FORBIDDEN).into_response();
    }

    match gen_qr_code(muid, new_uuid) {
        Ok(_) => {}
        Err(e) => return e.into_response(),
    }

    let image_bytes = tokio::fs::read("./output/styled_qr.png")
        .await
        .unwrap_or_else(|_| vec![]);

    (
        StatusCode::CREATED,
        [(header::CONTENT_TYPE, "image/png")],
        image_bytes,
    )
        .into_response()
}

fn gen_qr_code(muid: MUID, uuid: UUID) -> Result<(), StatusCode> {
    info!("Generating new QR for voter id {uuid} in meeting {muid}");
    let url = format!("{API_ENDPOINT}/login?muid=\"{muid}\"&uuid=\"{uuid}\"");
    info!("{url}");
    match generate_frameqr(
        &url,
        "../fsek-logo.jpg",
        Some(QrCodeEcc::High),
        Some(24),
        Some("output"),
        Some("styled_qr"),
        Some([255, 165, 0]), // Orange
        Some(4),             // Outer frame size
        Some(10),            // Inner frame size
        Some(FrameStyle::Rounded),
    ) {
        Ok(_) => Ok(()),
        Err(e) => {
            error!("Failed to create voter QR code: {e}");
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}
