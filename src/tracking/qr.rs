//! QR-code PNG generation.
//!
//! Port of the Go `internal/tracking/qr.go` (go-qrcode → the `qrcode` crate).
//! Encodes a payload to a 300×300 PNG at medium error-correction.

use crate::platform::errors::ApiError;

/// Encodes `payload` into a 300×300 PNG byte vector.
pub fn encode_png(payload: &str) -> Result<Vec<u8>, ApiError> {
    let code = qrcode::QrCode::with_error_correction_level(payload, qrcode::EcLevel::M)
        .map_err(|e| ApiError::internal(format!("qr encode: {e}")))?;
    let img = code
        .render::<image::Luma<u8>>()
        .min_dimensions(300, 300)
        .build();
    let mut buf = Vec::new();
    image::DynamicImage::ImageLuma8(img)
        .write_to(&mut std::io::Cursor::new(&mut buf), image::ImageFormat::Png)
        .map_err(|e| ApiError::internal(format!("qr png: {e}")))?;
    Ok(buf)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encodes_png_signature() {
        let png = encode_png("{\"id\":\"abc\"}").unwrap();
        // PNG magic number.
        assert_eq!(&png[..8], &[0x89, b'P', b'N', b'G', 0x0d, 0x0a, 0x1a, 0x0a]);
    }
}
