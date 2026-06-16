//! Clipboard image reading using arboard.
//!
//! Uses arboard crate for cross-platform clipboard image access.

use arboard::Clipboard;

/// Maximum image size in bytes (5 MB).
const MAX_IMAGE_BYTES: usize = 5 * 1024 * 1024;

/// Read an image from the clipboard.
/// Returns `Some(Vec<u8>)` with PNG bytes, or `None` if no image is available.
pub fn read_clipboard_image() -> Option<Vec<u8>> {
    let mut clipboard = Clipboard::new().ok()?;
    if let Ok(img) = clipboard.get_image() {
        // arboard returns RGBA bytes; encode to PNG
        let png_data = rgba_to_png(&img.bytes, img.width, img.height)?;
        if png_data.len() <= MAX_IMAGE_BYTES {
            Some(png_data)
        } else {
            None
        }
    } else {
        None
    }
}

/// Encode RGBA bytes as PNG.
fn rgba_to_png(rgba: &[u8], width: usize, height: usize) -> Option<Vec<u8>> {
    let mut buf = std::io::Cursor::new(Vec::new());
    {
        let mut encoder = png::Encoder::new(&mut buf, width as u32, height as u32);
        encoder.set_color(png::ColorType::Rgba);
        encoder.set_depth(png::BitDepth::Eight);
        let mut writer = encoder.write_header().ok()?;
        writer.write_image_data(rgba).ok()?;
    }
    Some(buf.into_inner())
}

/// Encode image bytes as a base64 data URI.
pub fn to_data_uri(bytes: &[u8]) -> String {
    use base64::{engine::general_purpose::STANDARD, Engine as _};
    let b64 = STANDARD.encode(bytes);
    format!("data:image/png;base64,{}", b64)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn to_data_uri_format() {
        let uri = to_data_uri(b"fake");
        assert!(uri.starts_with("data:image/png;base64,"));
    }
}
