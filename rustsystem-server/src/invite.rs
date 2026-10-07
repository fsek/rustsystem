//! Invite links and their QR codes (`docs/PROTOCOL.md` §4.2).
//!
//! `https://<server>/login?meeting=<meeting id>&invite=<one-time secret>`
//!
//! The link says nothing about whether the invitee is a host; that lives on the voter record.

use base64::{Engine, engine::general_purpose::STANDARD};
use qrcode::{EcLevel, QrCode, render::svg};
use serde::Serialize;

use rustsystem_core::{
    ApiError, ApiResult,
    internal::{MeetingId, VoterId},
};

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Invite {
    pub voter: VoterId,
    pub invite_link: String,
    /// An SVG `data:` URI, ready for an `<img src>`.
    pub qr_svg: String,
}

pub fn invite(public_url: &str, meeting: MeetingId, voter: VoterId, secret: &str) -> ApiResult<Invite> {
    let invite_link = format!("{public_url}/login?meeting={meeting}&invite={secret}");
    let code = QrCode::with_error_correction_level(invite_link.as_bytes(), EcLevel::M)
        .map_err(|e| ApiError::internal(format!("QR code: {e}")))?;
    let svg = code.render::<svg::Color>().min_dimensions(200, 200).build();
    Ok(Invite {
        voter,
        invite_link,
        qr_svg: format!("data:image/svg+xml;base64,{}", STANDARD.encode(svg)),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;

    #[test]
    fn link_carries_meeting_and_secret_only() {
        let (m, v) = (Uuid::new_v4(), Uuid::new_v4());
        let inv = invite("https://rosta.example", m, v, "s3cret").unwrap();
        assert_eq!(inv.invite_link, format!("https://rosta.example/login?meeting={m}&invite=s3cret"));
        assert!(!inv.invite_link.contains(&v.to_string()), "the voter ID is not a login secret");
        assert!(inv.qr_svg.starts_with("data:image/svg+xml;base64,"));
    }
}
