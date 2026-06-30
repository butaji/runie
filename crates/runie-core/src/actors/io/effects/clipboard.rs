//! Clipboard operations for IoActor using `arboard`.

/// Write text to system clipboard (blocking).
pub fn write_clipboard_sync(text: &str) -> bool {
    match arboard::Clipboard::new() {
        Ok(mut clipboard) => clipboard.set_text(text).is_ok(),
        Err(e) => {
            tracing::warn!("Failed to open clipboard: {}", e);
            false
        }
    }
}

/// Read text from system clipboard (blocking).
pub fn read_clipboard_sync() -> Result<String, String> {
    let mut clipboard =
        arboard::Clipboard::new().map_err(|e| format!("Failed to open clipboard: {}", e))?;
    clipboard
        .get_text()
        .map_err(|e| format!("Failed to read clipboard: {}", e))
}
