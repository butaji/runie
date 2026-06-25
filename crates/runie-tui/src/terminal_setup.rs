//! Terminal setup and progressive keyboard enhancement helpers.

use crate::terminal::caps;
use crossterm::event::{
    KeyboardEnhancementFlags, PopKeyboardEnhancementFlags, PushKeyboardEnhancementFlags,
};
use ratatui::{backend::CrosstermBackend, Terminal};
use std::io;

/// Set up the terminal and detect capabilities in one shot.
///
/// Capability detection is best-effort and intentionally does not fail
/// setup; a conservative capability set is returned if detection is
/// inconclusive.
pub fn setup_terminal() -> io::Result<(
    Terminal<CrosstermBackend<std::io::Stdout>>,
    caps::TerminalCapabilities,
)> {
    let capabilities = caps::detect_capabilities_from_env();

    crossterm::terminal::enable_raw_mode()?;
    let mut stdout = std::io::stdout();
    // Grok-style init: full mouse + focus + bracketed paste + sync update + cursor
    enable_mouse_grok_style(&mut stdout, &capabilities)?;
    // Progressive enhancement: ask the terminal to report modified keys.
    // We send both the kitty keyboard protocol and the xterm
    // modifyOtherKeys sequence so Shift+Enter is reported on the widest
    // range of terminals (kitty, Ghostty, WezTerm, iTerm2, xterm,
    // Termius, etc.). Unsupported terminals simply ignore these sequences.
    let _ = push_keyboard_enhancement_flags(&mut stdout);
    let terminal = Terminal::new(CrosstermBackend::new(stdout))?;
    Ok((terminal, capabilities))
}

pub fn push_keyboard_enhancement_flags<W: io::Write>(writer: &mut W) -> io::Result<()> {
    crossterm::execute!(
        writer,
        PushKeyboardEnhancementFlags(
            KeyboardEnhancementFlags::DISAMBIGUATE_ESCAPE_CODES
                | KeyboardEnhancementFlags::REPORT_ALL_KEYS_AS_ESCAPE_CODES
                | KeyboardEnhancementFlags::REPORT_EVENT_TYPES
                | KeyboardEnhancementFlags::REPORT_ALTERNATE_KEYS
        ),
    )?;
    push_xterm_modify_other_keys(writer)
}

/// Enable xterm `modifyOtherKeys` level 2 so modified keys such as
/// Shift+Enter are sent as CSI sequences that crossterm can parse.
fn push_xterm_modify_other_keys<W: io::Write>(writer: &mut W) -> io::Result<()> {
    writer.write_all(b"\x1b[>4;2m")?;
    writer.flush()
}

/// Reset xterm `modifyOtherKeys` to its default level.
pub fn reset_xterm_modify_other_keys<W: io::Write>(writer: &mut W) -> io::Result<()> {
    writer.write_all(b"\x1b[>4;0m")?;
    writer.flush()
}

/// Pop all progressive keyboard enhancements, including kitty protocol
/// flags and xterm modifyOtherKeys.
pub fn reset_keyboard_enhancements<W: io::Write>(writer: &mut W) -> io::Result<()> {
    crossterm::execute!(writer, PopKeyboardEnhancementFlags)?;
    reset_xterm_modify_other_keys(writer)
}

pub fn restore_terminal_graphics<W: io::Write>(
    writer: &mut W,
    capabilities: caps::TerminalCapabilities,
) -> io::Result<()> {
    enable_mouse_grok_style(writer, &capabilities)?;
    let _ = push_keyboard_enhancement_flags(writer);
    Ok(())
}

fn mouse_sequence(cap: caps::MouseCapability) -> Option<&'static [u8]> {
    match cap {
        caps::MouseCapability::None => None,
        caps::MouseCapability::Legacy => Some(b"\x1b[?1000h"),
        caps::MouseCapability::Sgr => Some(b"\x1b[?1006h"),
        caps::MouseCapability::SgrExtended => Some(b"\x1b[?1006;6h"),
    }
}

/// Write the mouse mode escape sequence for the given capability level.
pub fn enable_mouse<W: io::Write>(writer: &mut W, caps: caps::MouseCapability) -> io::Result<()> {
    if let Some(seq) = mouse_sequence(caps) {
        writer.write_all(seq)?;
        writer.flush()
    } else {
        Ok(())
    }
}

fn enter_alternate_screen<W: io::Write>(writer: &mut W) -> io::Result<()> {
    writer.write_all(b"\x1b[?1049h")
}

const GROK_MOUSE_SEQUENCES: &[&[u8]] = &[
    b"\x1b[?1000h",
    b"\x1b[?1002h",
    b"\x1b[?1003h",
    b"\x1b[?1015h",
    b"\x1b[?1006h",
];

fn write_grok_mouse_modes<W: io::Write>(
    writer: &mut W,
    caps: &caps::TerminalCapabilities,
) -> io::Result<()> {
    if caps.mouse == caps::MouseCapability::None {
        return Ok(());
    }
    for seq in GROK_MOUSE_SEQUENCES {
        writer.write_all(seq)?;
    }
    Ok(())
}

fn enable_focus_tracking<W: io::Write>(
    writer: &mut W,
    caps: &caps::TerminalCapabilities,
) -> io::Result<()> {
    if caps.focus_tracking {
        writer.write_all(b"\x1b[?1004h")?;
    }
    Ok(())
}

fn enable_bracketed_paste<W: io::Write>(writer: &mut W) -> io::Result<()> {
    writer.write_all(b"\x1b[?2004h")
}

fn begin_sync_update<W: io::Write>(writer: &mut W) -> io::Result<()> {
    writer.write_all(b"\x1b[?2026h")
}

fn hide_cursor_and_set_block<W: io::Write>(writer: &mut W) -> io::Result<()> {
    writer.write_all(b"\x1b[?25l")?;
    writer.write_all(b"\x1b[1 q")?;
    Ok(())
}

/// Write the full Grok-style mouse + terminal init sequence.
/// Only the mouse modes are gated on `caps.mouse != None`; focus tracking,
/// bracketed paste, and synchronized updates are emitted unconditionally
/// (unsupported terminals ignore them).
pub fn enable_mouse_grok_style<W: io::Write>(
    writer: &mut W,
    caps: &caps::TerminalCapabilities,
) -> io::Result<()> {
    enter_alternate_screen(writer)?;
    write_grok_mouse_modes(writer, caps)?;
    enable_focus_tracking(writer, caps)?;
    enable_bracketed_paste(writer)?;
    begin_sync_update(writer)?;
    hide_cursor_and_set_block(writer)?;
    writer.flush()
}

fn end_sync_update<W: io::Write>(writer: &mut W) -> io::Result<()> {
    writer.write_all(b"\x1b[?2026l")
}

fn show_cursor<W: io::Write>(writer: &mut W) -> io::Result<()> {
    writer.write_all(b"\x1b[?25h")
}

fn disable_bracketed_paste<W: io::Write>(writer: &mut W) -> io::Result<()> {
    writer.write_all(b"\x1b[?2004l")
}

fn disable_focus_tracking<W: io::Write>(writer: &mut W) -> io::Result<()> {
    writer.write_all(b"\x1b[?1004l")
}

fn disable_all_mouse_modes<W: io::Write>(writer: &mut W) -> io::Result<()> {
    writer.write_all(b"\x1b[?1003l")?;
    writer.write_all(b"\x1b[?1002l")?;
    writer.write_all(b"\x1b[?1000l")?;
    Ok(())
}

fn leave_alternate_screen<W: io::Write>(writer: &mut W) -> io::Result<()> {
    writer.write_all(b"\x1b[?1049l")
}

/// Disable all Grok-style terminal modes.
pub fn disable_mouse_grok_style<W: io::Write>(writer: &mut W) -> io::Result<()> {
    end_sync_update(writer)?;
    show_cursor(writer)?;
    disable_bracketed_paste(writer)?;
    disable_focus_tracking(writer)?;
    disable_all_mouse_modes(writer)?;
    leave_alternate_screen(writer)?;
    writer.flush()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn enable_mouse_none_writes_nothing() {
        let mut buf = Vec::new();
        enable_mouse(&mut buf, caps::MouseCapability::None).unwrap();
        assert!(buf.is_empty(), "None mode should write no bytes");
    }

    #[test]
    fn enable_mouse_legacy_emits_csi_1000() {
        let mut buf = Vec::new();
        enable_mouse(&mut buf, caps::MouseCapability::Legacy).unwrap();
        assert_eq!(buf, b"\x1b[?1000h");
    }

    #[test]
    fn enable_mouse_sgr_emits_csi_1006() {
        let mut buf = Vec::new();
        enable_mouse(&mut buf, caps::MouseCapability::Sgr).unwrap();
        assert_eq!(buf, b"\x1b[?1006h");
    }

    #[test]
    fn enable_mouse_sgr_extended_emits_csi_1006_6() {
        let mut buf = Vec::new();
        enable_mouse(&mut buf, caps::MouseCapability::SgrExtended).unwrap();
        assert_eq!(buf, b"\x1b[?1006;6h");
    }

    // ── Grok-style init tests ──────────────────────────────────────────────

    #[test]
    fn mouse_init_sequence_includes_all_grok_modes() {
        let mut buf = Vec::new();
        let caps = caps::TerminalCapabilities {
            mouse: caps::MouseCapability::Sgr,
            focus_tracking: true,
            ..Default::default()
        };
        enable_mouse_grok_style(&mut buf, &caps).unwrap();
        let s = String::from_utf8(buf).unwrap();
        assert!(s.contains("\x1b[?1000h"), "missing ?1000h");
        assert!(s.contains("\x1b[?1002h"), "missing ?1002h");
        assert!(s.contains("\x1b[?1003h"), "missing ?1003h");
        assert!(s.contains("\x1b[?1015h"), "missing ?1015h");
        assert!(s.contains("\x1b[?1006h"), "missing ?1006h");
        assert!(s.contains("\x1b[?1004h"), "missing ?1004h (focus)");
        assert!(
            s.contains("\x1b[?2004h"),
            "missing ?2004h (bracketed paste)"
        );
        assert!(s.contains("\x1b[?2026h"), "missing ?2026h (sync update)");
        assert!(
            s.contains("\x1b[?1049h"),
            "missing ?1049h (alternate screen)"
        );
        assert!(s.contains("\x1b[?25l"), "missing ?25l (hide cursor)");
        assert!(s.contains("\x1b[1 q"), "missing block cursor");
    }

    #[test]
    fn mouse_init_omits_mouse_when_capability_is_none() {
        let mut buf = Vec::new();
        let caps = caps::TerminalCapabilities {
            mouse: caps::MouseCapability::None,
            focus_tracking: false,
            ..Default::default()
        };
        enable_mouse_grok_style(&mut buf, &caps).unwrap();
        let s = String::from_utf8(buf).unwrap();
        assert!(!s.contains("?1000"), "should not emit mouse modes");
        assert!(!s.contains("?1002"), "should not emit mouse modes");
        assert!(!s.contains("?1003"), "should not emit mouse modes");
    }

    #[test]
    fn cleanup_sequence_disables_all_modes() {
        let mut buf = Vec::new();
        disable_mouse_grok_style(&mut buf).unwrap();
        let s = String::from_utf8(buf).unwrap();
        assert!(s.contains("\x1b[?2026l"), "missing ?2026l (sync end)");
        assert!(s.contains("\x1b[?25h"), "missing ?25h (show cursor)");
        assert!(
            s.contains("\x1b[?2004l"),
            "missing ?2004l (bracketed paste)"
        );
        assert!(s.contains("\x1b[?1004l"), "missing ?1004l (focus)");
        assert!(s.contains("\x1b[?1003l"), "missing ?1003l");
        assert!(s.contains("\x1b[?1002l"), "missing ?1002l");
        assert!(s.contains("\x1b[?1000l"), "missing ?1000l");
        assert!(s.contains("\x1b[?1049l"), "missing ?1049l (exit alternate)");
    }
}
