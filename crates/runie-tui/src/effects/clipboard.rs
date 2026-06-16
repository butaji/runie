//! Clipboard effect handlers.

use crate::terminal::{caps::TerminalCapabilities, clipboard as term_clipboard};
use runie_core::model::Role;
use runie_core::ChatMessage;

/// Copy the given text to the terminal clipboard.
/// Uses OSC 52 when the terminal supports it; otherwise falls back to
/// a platform-specific command-line tool (pbcopy / wl-copy / xclip / clip).
pub fn copy_to_clipboard(text: String, caps: TerminalCapabilities) {
    if caps.clipboard {
        if term_clipboard::copy_to_clipboard(&mut std::io::stdout(), &text).is_ok() {
            return;
        }
    }
    let _ = platform_copy(&text);
}

/// Copy the last assistant response to the terminal clipboard.
pub fn copy_last_response(messages: Vec<ChatMessage>, caps: TerminalCapabilities) {
    let text = messages
        .iter()
        .rev()
        .find(|m| m.role == Role::Assistant)
        .map(|m| m.content.clone())
        .unwrap_or_default();
    if text.is_empty() {
        return;
    }
    if caps.clipboard {
        if term_clipboard::copy_to_clipboard(&mut std::io::stdout(), &text).is_ok() {
            return;
        }
    }
    let _ = platform_copy(&text);
}

/// Write text to the system clipboard using a platform-specific command.
/// Returns `Ok(())` if the command succeeded, `Err(String)` with the
/// error message otherwise.
fn platform_copy(text: &str) -> Result<(), String> {
    #[cfg(target_os = "macos")]
    {
        return platform_copy_macos(text);
    }

    #[cfg(all(unix, not(target_os = "macos")))]
    {
        return platform_copy_unix(text);
    }

    #[cfg(target_os = "windows")]
    {
        return platform_copy_windows(text);
    }

    #[cfg(not(any(target_os = "macos", unix, target_os = "windows")))]
    {
        #[allow(clippy::unreachable)]
        return Err("no clipboard fallback available for this platform".into());
    }
}

#[cfg(target_os = "macos")]
fn platform_copy_macos(text: &str) -> Result<(), String> {
    use std::io::Write;
    use std::process::{Command, Stdio};
    let mut child = Command::new("pbcopy")
        .stdin(Stdio::piped())
        .spawn()
        .map_err(|e| format!("pbcopy failed: {}", e))?;
    if let Some(mut stdin) = child.stdin.take() {
        stdin
            .write_all(text.as_bytes())
            .map_err(|e| format!("pbcopy write failed: {}", e))?;
    }
    child
        .wait()
        .map_err(|e| format!("pbcopy wait failed: {}", e))?;
    Ok(())
}

#[cfg(all(unix, not(target_os = "macos")))]
fn platform_copy_unix(text: &str) -> Result<(), String> {
    if xcp_command("wl-copy", text).is_ok() {
        return Ok(());
    }
    if xcp_command("xclip", text).is_ok() {
        return Ok(());
    }
    Err("no clipboard tool found (tried wl-copy, xclip)".into())
}

#[cfg(target_os = "windows")]
fn platform_copy_windows(text: &str) -> Result<(), String> {
    use std::process::Command;
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

#[cfg(all(unix, not(target_os = "macos")))]
fn xcp_command(cmd: &str, text: &str) -> Result<(), String> {
    use std::io::Write;
    use std::process::{Command, Stdio};
    let mut child = Command::new(cmd)
        .stdin(Stdio::piped())
        .spawn()
        .map_err(|e| format!("{} failed: {}", cmd, e))?;
    if let Some(mut stdin) = child.stdin.take() {
        stdin
            .write_all(text.as_bytes())
            .map_err(|e| format!("{} write failed: {}", cmd, e))?;
    }
    child
        .wait()
        .map_err(|e| format!("{} wait failed: {}", cmd, e))?;
    Ok(())
}
