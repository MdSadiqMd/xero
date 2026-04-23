use std::io::Cursor;

use image::codecs::jpeg::JpegEncoder;
use image::{ColorType, ImageEncoder};

/// JPEG quality (0-100). Q80 matches the plan — visually indistinguishable
/// from lossless for UI streaming, ~10x smaller than PNG.
pub const JPEG_QUALITY: u8 = 80;

#[derive(Debug, thiserror::Error)]
pub enum CodecError {
    #[error("rgba buffer size {actual} does not match {width}x{height} ({expected} bytes)")]
    InvalidRgbaSize {
        width: u32,
        height: u32,
        expected: usize,
        actual: usize,
    },
    #[error("jpeg encode failed: {0}")]
    JpegEncode(String),
}

/// Encode an `RGBA8` image buffer as JPEG. The alpha channel is dropped
/// (JPEG doesn't carry one), which is fine for our opaque device framebuffer.
pub fn encode_jpeg_rgba(rgba: &[u8], width: u32, height: u32) -> Result<Vec<u8>, CodecError> {
    let expected = (width as usize)
        .checked_mul(height as usize)
        .and_then(|n| n.checked_mul(4))
        .ok_or(CodecError::InvalidRgbaSize {
            width,
            height,
            expected: 0,
            actual: rgba.len(),
        })?;

    if rgba.len() != expected {
        return Err(CodecError::InvalidRgbaSize {
            width,
            height,
            expected,
            actual: rgba.len(),
        });
    }

    // JPEG has no alpha channel — drop it. Over an opaque framebuffer this is
    // lossless; for translucent overlays the plan assumes a solid background.
    let mut rgb = Vec::with_capacity(rgba.len() / 4 * 3);
    for chunk in rgba.chunks_exact(4) {
        rgb.push(chunk[0]);
        rgb.push(chunk[1]);
        rgb.push(chunk[2]);
    }

    let mut out = Vec::with_capacity(rgb.len() / 4);
    let encoder = JpegEncoder::new_with_quality(Cursor::new(&mut out), JPEG_QUALITY);
    encoder
        .write_image(&rgb, width, height, ColorType::Rgb8.into())
        .map_err(|e| CodecError::JpegEncode(e.to_string()))?;
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trip_decodes_to_expected_dimensions() {
        let width = 32;
        let height = 16;
        let mut rgba = Vec::with_capacity((width * height * 4) as usize);
        for y in 0..height {
            for x in 0..width {
                rgba.push((x * 8) as u8);
                rgba.push((y * 16) as u8);
                rgba.push(((x + y) * 4) as u8);
                rgba.push(255);
            }
        }

        let jpeg = encode_jpeg_rgba(&rgba, width, height).expect("encode");
        // JPEG magic header: 0xFF 0xD8 0xFF
        assert_eq!(&jpeg[..3], &[0xFF, 0xD8, 0xFF]);

        let decoded =
            image::load_from_memory_with_format(&jpeg, image::ImageFormat::Jpeg).expect("decode");
        assert_eq!(decoded.width(), width);
        assert_eq!(decoded.height(), height);
    }

    #[test]
    fn rejects_mismatched_rgba_length() {
        let err = encode_jpeg_rgba(&[0; 10], 4, 4).unwrap_err();
        match err {
            CodecError::InvalidRgbaSize {
                width,
                height,
                expected,
                actual,
            } => {
                assert_eq!(width, 4);
                assert_eq!(height, 4);
                assert_eq!(expected, 64);
                assert_eq!(actual, 10);
            }
            _ => panic!("expected InvalidRgbaSize, got {err:?}"),
        }
    }
}
