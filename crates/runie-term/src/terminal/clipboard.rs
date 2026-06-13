//! Terminal clipboard integration via OSC 52.
//!
//! OSC 52 lets a terminal application copy text to the system clipboard
//! without spawning external tools. It works across SSH and inside tmux
//! when passthrough is enabled (`set -g allow-passthrough on`).
//!
//! This module intentionally writes raw escape sequences to the active
//! terminal (stdout). It is not unit-testable for actual clipboard
//! side-effects, but the encoding logic is pure and testable.

use std::io::{self, Write};

/// Encode text as an OSC 52 "set clipboard" sequence targeting the
/// system clipboard (`c`). Returns the raw bytes so callers can choose
/// where to write them.
pub fn osc52_clipboard_sequence(text: &str) -> Vec<u8> {
    use base64::{engine::general_purpose::STANDARD, Engine};
    let encoded = STANDARD.encode(text.as_bytes());
    // OSC 52; set selection clipboard: ESC ] 52 ; c ; <base64> ESC \
    format!("\x1b]52;c;{}\x1b\\", encoded).into_bytes()
}

/// Encode text as an OSC 52 "set primary selection" sequence (`p`).
pub fn osc52_primary_sequence(text: &str) -> Vec<u8> {
    use base64::{engine::general_purpose::STANDARD, Engine};
    let encoded = STANDARD.encode(text.as_bytes());
    format!("\x1b]52;p;{}\x1b\\", encoded).into_bytes()
}

/// Set the terminal window title via OSC 0/2.
pub fn set_terminal_title(title: &str) -> Vec<u8> {
    // OSC 0 is the icon name+window title; OSC 2 is window title only.
    format!("\x1b]0;{}\x1b\\", title).into_bytes()
}

/// Copy `text` to the system clipboard by writing the OSC 52 sequence
/// to `writer`. Best-effort: errors are returned but ignored by callers
/// in the render path.
#[allow(dead_code)]
pub fn copy_to_clipboard<W: Write>(writer: &mut W, text: &str) -> io::Result<()> {
    writer.write_all(&osc52_clipboard_sequence(text))?;
    writer.flush()
}

/// Copy `text` to the primary selection.
#[allow(dead_code)]
pub fn copy_to_primary<W: Write>(writer: &mut W, text: &str) -> io::Result<()> {
    writer.write_all(&osc52_primary_sequence(text))?;
    writer.flush()
}

/// Update the terminal window title.
#[allow(dead_code)]
pub fn update_title<W: Write>(writer: &mut W, title: &str) -> io::Result<()> {
    writer.write_all(&set_terminal_title(title))?;
    writer.flush()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn osc52_clipboard_prefix_and_suffix() {
        let seq = osc52_clipboard_sequence("hello");
        let s = String::from_utf8(seq).unwrap();
        assert!(s.starts_with("\x1b]52;c;"));
        assert!(s.ends_with("\x1b\\"));
    }

    #[test]
    fn osc52_clipboard_base64_payload() {
        use base64::{engine::general_purpose::STANDARD, Engine};
        let seq = osc52_clipboard_sequence("hello");
        let s = String::from_utf8(seq).unwrap();
        let prefix = "\x1b]52;c;";
        let suffix = "\x1b\\";
        let payload = &s[prefix.len()..s.len() - suffix.len()];
        assert_eq!(payload, STANDARD.encode("hello"));
    }

    #[test]
    fn osc52_primary_uses_p_selection() {
        let seq = osc52_primary_sequence("hello");
        let s = String::from_utf8(seq).unwrap();
        assert!(s.starts_with("\x1b]52;p;"));
    }

    #[test]
    fn set_terminal_title_emits_osc_0() {
        let seq = set_terminal_title("runie");
        let s = String::from_utf8(seq).unwrap();
        assert_eq!(s, "\x1b]0;runie\x1b\\");
    }

    #[test]
    fn copy_to_clipboard_writes_sequence() {
        let mut buf = Vec::new();
        copy_to_clipboard(&mut buf, "abc").unwrap();
        assert_eq!(buf, osc52_clipboard_sequence("abc"));
    }

    #[test]
    fn update_title_writes_sequence() {
        let mut buf = Vec::new();
        update_title(&mut buf, "runie").unwrap();
        assert_eq!(buf, set_terminal_title("runie"));
    }

    #[test]
    fn empty_string_is_encoded() {
        let seq = osc52_clipboard_sequence("");
        let s = String::from_utf8(seq).unwrap();
        assert_eq!(s, "\x1b]52;c;\x1b\\");
    }
}
