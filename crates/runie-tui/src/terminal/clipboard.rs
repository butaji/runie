#![allow(dead_code, reason = "standalone module for future clipboard integration")]

//! Terminal clipboard integration via OSC 52.
//!
//! OSC 52 (Set clipboard) is supported by most modern terminals:
//! - iTerm2, WezTerm, Kitty, Alacritty, Windows Terminal, VS Code, etc.
//! - tmux (with passthrough mode)
//!
//! **Status**: Standalone module, not yet wired to TUI input handlers.
//! Kept for future `/copy` slash command and `copy_to_clipboard` tool integration.

use base64::{engine::general_purpose::STANDARD, Engine};
use std::io::Write;

/// Copy text to the clipboard via OSC 52.
pub fn copy_to_clipboard<W: Write>(writer: &mut W, text: &str) -> std::io::Result<()> {
    let encoded = STANDARD.encode(text.as_bytes());
    // OSC 52: Copy to clipboard selection
    // Format: ESC ] 52 ; c ; <base64> BEL ESC \
    writer.write_all(format!("\x1b]52;c;{}\x07", encoded).as_bytes())?;
    writer.flush()
}

/// Copy to primary selection (X11/Linux).
pub fn copy_to_primary<W: Write>(writer: &mut W, text: &str) -> std::io::Result<()> {
    let encoded = STANDARD.encode(text.as_bytes());
    // OSC 52 with 'p' for primary selection
    writer.write_all(format!("\x1b]52;p;{}\x07", encoded).as_bytes())?;
    writer.flush()
}

/// Set terminal title via OSC 0/2.
pub fn set_terminal_title<W: Write>(writer: &mut W, title: &str) -> std::io::Result<()> {
    // OSC 0: Set window icon name and title
    writer.write_all(format!("\x1b]0;{}\x07", title).as_bytes())?;
    writer.flush()
}

/// Set cursor color via OSC 12.
pub fn set_cursor_color<W: Write>(writer: &mut W, color: &str) -> std::io::Result<()> {
    // OSC 12: Set cursor color
    writer.write_all(format!("\x1b]12;{}\x07", color).as_bytes())?;
    writer.flush()
}

/// Clear the terminal screen.
pub fn clear_screen<W: Write>(writer: &mut W) -> std::io::Result<()> {
    writer.write_all(b"\x1b[2J")?;
    writer.flush()
}

/// Move cursor to home position (1,1).
pub fn cursor_home<W: Write>(writer: &mut W) -> std::io::Result<()> {
    writer.write_all(b"\x1b[H")?;
    writer.flush()
}

/// Show cursor.
pub fn show_cursor<W: Write>(writer: &mut W) -> std::io::Result<()> {
    writer.write_all(b"\x1b[?25h")?;
    writer.flush()
}

/// Hide cursor.
pub fn hide_cursor<W: Write>(writer: &mut W) -> std::io::Result<()> {
    writer.write_all(b"\x1b[?25l")?;
    writer.flush()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn copy_to_clipboard_writes_osc52_sequence() {
        let mut buf = Vec::new();
        copy_to_clipboard(&mut buf, "hello").unwrap();
        let output = String::from_utf8(buf).unwrap();
        assert!(output.starts_with("\x1b]52;c;"));
        assert!(output.ends_with("\x07"));
        // "hello" is base64 encoded
        assert!(output.contains("aGVsbG8="));
    }

    #[test]
    fn copy_to_primary_writes_osc52_primary_sequence() {
        let mut buf = Vec::new();
        copy_to_primary(&mut buf, "world").unwrap();
        let output = String::from_utf8(buf).unwrap();
        assert!(output.starts_with("\x1b]52;p;"));
        assert!(output.ends_with("\x07"));
    }

    #[test]
    fn set_terminal_title_writes_osc0() {
        let mut buf = Vec::new();
        set_terminal_title(&mut buf, "My Title").unwrap();
        let output = String::from_utf8(buf).unwrap();
        assert!(output.starts_with("\x1b]0;"));
        assert!(output.ends_with("\x07"));
        assert!(output.contains("My Title"));
    }

    #[test]
    fn set_cursor_color_writes_osc12() {
        let mut buf = Vec::new();
        set_cursor_color(&mut buf, "#ff00ff").unwrap();
        let output = String::from_utf8(buf).unwrap();
        assert!(output.starts_with("\x1b]12;"));
        assert!(output.ends_with("\x07"));
        assert!(output.contains("#ff00ff"));
    }

    #[test]
    fn clear_screen_writes_ansi_sequence() {
        let mut buf = Vec::new();
        clear_screen(&mut buf).unwrap();
        assert_eq!(buf, b"\x1b[2J");
    }

    #[test]
    fn cursor_home_writes_ansi_sequence() {
        let mut buf = Vec::new();
        cursor_home(&mut buf).unwrap();
        assert_eq!(buf, b"\x1b[H");
    }

    #[test]
    fn show_cursor_writes_ansi_sequence() {
        let mut buf = Vec::new();
        show_cursor(&mut buf).unwrap();
        assert_eq!(buf, b"\x1b[?25h");
    }

    #[test]
    fn hide_cursor_writes_ansi_sequence() {
        let mut buf = Vec::new();
        hide_cursor(&mut buf).unwrap();
        assert_eq!(buf, b"\x1b[?25l");
    }

    #[test]
    fn empty_string_copy() {
        let mut buf = Vec::new();
        copy_to_clipboard(&mut buf, "").unwrap();
        let output = String::from_utf8(buf).unwrap();
        assert_eq!(output, "\x1b]52;c;\x07");
    }

    #[test]
    fn unicode_copy() {
        let mut buf = Vec::new();
        copy_to_clipboard(&mut buf, "🦀").unwrap();
        let output = String::from_utf8(buf).unwrap();
        assert!(output.starts_with("\x1b]52;c;"));
        assert!(output.ends_with("\x07"));
    }
}
