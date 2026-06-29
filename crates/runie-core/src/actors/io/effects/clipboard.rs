//! Clipboard operations for IoActor.

use std::io::Write;
use std::process::{Command, Stdio};

/// Write text to system clipboard (blocking).
pub fn write_clipboard_sync(text: &str) -> bool {
    #[cfg(target_os = "macos")]
    {
        write_clipboard_macos(text).is_ok()
    }
    #[cfg(all(unix, not(target_os = "macos")))]
    {
        write_clipboard_wl_copy(text).is_ok() || write_clipboard_xclip(text).is_ok()
    }
    #[cfg(target_os = "windows")]
    {
        write_clipboard_windows(text).is_ok()
    }
    #[cfg(not(any(target_os = "macos", unix, target_os = "windows")))]
    {
        false
    }
}

/// Read text from system clipboard (blocking).
pub fn read_clipboard_sync() -> Result<String, String> {
    #[cfg(target_os = "macos")]
    {
        read_clipboard_macos()
    }
    #[cfg(all(unix, not(target_os = "macos")))]
    {
        read_clipboard_wl_paste().or_else(read_clipboard_xsel)
    }
    #[cfg(target_os = "windows")]
    {
        read_clipboard_windows()
    }
    #[cfg(not(any(target_os = "macos", unix, target_os = "windows")))]
    {
        Err("clipboard not supported on this platform".to_string())
    }
}

#[cfg(target_os = "macos")]
fn write_clipboard_macos(text: &str) -> Result<(), String> {
    let mut child = Command::new("pbcopy")
        .stdin(Stdio::piped())
        .spawn()
        .map_err(|e| format!("pbcopy failed: {}", e))?;
    if let Some(mut stdin) = child.stdin.take() {
        stdin
            .write_all(text.as_bytes())
            .map_err(|e| format!("write failed: {}", e))?;
    }
    child.wait().map_err(|e| format!("wait failed: {}", e))?;
    Ok(())
}

#[cfg(target_os = "macos")]
fn read_clipboard_macos() -> Result<String, String> {
    let output = Command::new("pbpaste")
        .output()
        .map_err(|e| format!("pbpaste failed: {}", e))?;
    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    } else {
        Err("pbpaste exited with non-zero status".to_string())
    }
}

#[cfg(all(unix, not(target_os = "macos")))]
fn write_clipboard_wl_copy(text: &str) -> Result<(), String> {
    let mut child = Command::new("wl-copy")
        .stdin(Stdio::piped())
        .spawn()
        .map_err(|e| format!("wl-copy failed: {}", e))?;
    if let Some(mut stdin) = child.stdin.take() {
        stdin
            .write_all(text.as_bytes())
            .map_err(|e| format!("write failed: {}", e))?;
    }
    child.wait().map_err(|e| format!("wait failed: {}", e))?;
    Ok(())
}

#[cfg(all(unix, not(target_os = "macos")))]
fn write_clipboard_xclip(text: &str) -> Result<(), String> {
    let mut child = Command::new("xclip")
        .args(["-selection", "clipboard"])
        .stdin(Stdio::piped())
        .spawn()
        .map_err(|e| format!("xclip failed: {}", e))?;
    if let Some(mut stdin) = child.stdin.take() {
        stdin
            .write_all(text.as_bytes())
            .map_err(|e| format!("write failed: {}", e))?;
    }
    child.wait().map_err(|e| format!("wait failed: {}", e))?;
    Ok(())
}

#[cfg(all(unix, not(target_os = "macos")))]
fn read_clipboard_wl_paste() -> Result<String, String> {
    let output = Command::new("wl-paste")
        .output()
        .map_err(|e| format!("wl-paste failed: {}", e))?;
    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    } else {
        Err("wl-paste exited with non-zero status".to_string())
    }
}

#[cfg(all(unix, not(target_os = "macos")))]
fn read_clipboard_xsel() -> Result<String, String> {
    let output = Command::new("xsel")
        .args(["--clipboard"])
        .output()
        .map_err(|e| format!("xsel failed: {}", e))?;
    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    } else {
        Err("xsel exited with non-zero status".to_string())
    }
}

#[cfg(target_os = "windows")]
fn write_clipboard_windows(text: &str) -> Result<(), String> {
    let out = Command::new("cmd")
        .args(["/C", "echo", text, "| clip"])
        .output()
        .map_err(|e| format!("clip failed: {}", e))?;
    if out.status.success() {
        Ok(())
    } else {
        Err(format!("clip exited with {:?}", out.status))
    }
}

#[cfg(target_os = "windows")]
fn read_clipboard_windows() -> Result<String, String> {
    Err("clipboard read not yet supported on Windows".to_string())
}
