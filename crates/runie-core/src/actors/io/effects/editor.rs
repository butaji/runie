//! External editor handling.

use std::io::Write;

/// Open external editor and return edited text (blocking).
pub fn open_editor_sync(text: String) -> Result<String, String> {
    let editor = std::env::var("EDITOR")
        .unwrap_or_else(|_| if cfg!(windows) { "notepad" } else { "vi" }.to_owned());

    let mut tmp = tempfile::NamedTempFile::new()
        .map_err(|e| format!("temp file error: {}", e))?;
    tmp.write_all(text.as_bytes())
        .map_err(|e| format!("write error: {}", e))?;
    tmp.flush().map_err(|e| format!("flush error: {}", e))?;
    let path = tmp.into_temp_path();

    let status = std::process::Command::new(&editor)
        .arg(&path)
        .status()
        .map_err(|e| format!("editor error: {}", e))?;

    if status.success() {
        let content = std::fs::read_to_string(&path).unwrap_or_default();
        Ok(content)
    } else {
        Err("editor exited with non-zero status".to_string())
    }
}
