use std::error::Error;

use qirust::helper::{FrameStyle, generate_frameqr};
use qirust::qrcode::QrCodeEcc;

pub fn gen_qr_code() -> Result<(), Box<dyn Error>> {
    generate_frameqr(
        "https://127.0.0.1/login?cred=placeholder",
        "../fsek-logo.jpg",
        Some(QrCodeEcc::High),
        Some(24),
        Some("output"),
        Some("styled_qr"),
        Some([255, 165, 0]), // Orange
        Some(4),             // Outer frame size
        Some(10),            // Inner frame size
        Some(FrameStyle::Rounded),
    )?;

    Ok(())
}
