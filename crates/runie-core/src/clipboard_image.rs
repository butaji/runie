//! Clipboard image reading — platform-specific implementations.
//!
//! Reads PNG images from the system clipboard and returns them as raw bytes.
//! Falls back to None if the clipboard contains text or no image.

/// Maximum image size in bytes (5 MB).
const MAX_IMAGE_BYTES: usize = 5 * 1024 * 1024;

/// Read an image from the clipboard.
/// Returns `Some(Vec<u8>)` with PNG bytes, or `None` if no image is available.
pub fn read_clipboard_image() -> Option<Vec<u8>> {
    #[cfg(target_os = "macos")]
    {
        read_macos()
    }
    #[cfg(target_os = "linux")]
    {
        read_linux()
    }
    #[cfg(target_os = "windows")]
    {
        read_windows()
    }
    #[cfg(not(any(target_os = "macos", target_os = "linux", target_os = "windows")))]
    {
        None
    }
}

#[cfg(target_os = "macos")]
fn read_macos() -> Option<Vec<u8>> {
    // Try to read PNG data from clipboard using osascript
    match std::process::Command::new("osascript")
        .args([
            "-e",
            "try",
            "-e",
            "set pngData to (the clipboard as «class PNGf»)",
            "-e",
            "set fileRef to (open for access POSIX path \"/tmp/runie_clipboard.png\" with write permission)",
            "-e",
            "write pngData to fileRef",
            "-e",
            "close access fileRef",
            "-e",
            "return \"ok\"",
            "-e",
            "on error errMsg",
            "-e",
            "return errMsg",
            "-e",
            "end try",
        ])
        .output()
    {
        Ok(output) => {
            let stdout = String::from_utf8_lossy(&output.stdout);
            if stdout.trim() == "ok" {
                if let Ok(data) = std::fs::read("/tmp/runie_clipboard.png") {
                    if !data.is_empty() && data.len() <= MAX_IMAGE_BYTES {
                        let _ = std::fs::remove_file("/tmp/runie_clipboard.png");
                        return Some(data);
                    }
                }
            }
            None
        }
        Err(_) => None,
    }
}

#[cfg(target_os = "linux")]
fn read_linux() -> Option<Vec<u8>> {
    // Try wl-paste (Wayland) first, then xclip (X11)
    let ways = [
        ("wl-paste", vec!["--type", "image/png"]),
        ("xclip", vec!["-selection", "clipboard", "-t", "image/png", "-o"]),
    ];
    for (cmd, args) in &ways {
        if let Ok(output) = std::process::Command::new(cmd).args(args).output() {
            if output.status.success() && !output.stdout.is_empty() && output.stdout.len() <= MAX_IMAGE_BYTES {
                return Some(output.stdout);
            }
        }
    }
    None
}

#[cfg(target_os = "windows")]
fn read_windows() -> Option<Vec<u8>> {
    // Placeholder: would use clipboard-win or arboard crate
    None
}

/// Encode image bytes as a base64 data URI.
pub fn to_data_uri(bytes: &[u8]) -> String {
    use base64::{Engine as _, engine::general_purpose::STANDARD};
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
