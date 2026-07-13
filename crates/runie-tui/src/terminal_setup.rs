//! Terminal setup and progressive keyboard enhancement helpers.
//!
//! Uses `crossterm` commands for standard terminal sequences.
//! Non-standard sequences (xterm modifyOtherKeys, extended mouse modes,
//! synchronized update) are kept as raw bytes.

use crate::terminal::caps;
use crossterm::{
    cursor::{Hide, SetCursorStyle, Show},
    event::{
        DisableBracketedPaste, DisableFocusChange, DisableMouseCapture, EnableBracketedPaste,
        EnableFocusChange, EnableMouseCapture, KeyboardEnhancementFlags,
        PopKeyboardEnhancementFlags, PushKeyboardEnhancementFlags,
    },
    terminal::{EnterAlternateScreen, LeaveAlternateScreen},
    QueueableCommand,
};
use ratatui::{backend::CrosstermBackend, Terminal};
use std::io;

/// Set up the terminal and detect capabilities in one shot.
///
/// Capability detection is best-effort and intentionally does not fail
/// setup; a conservative capability set is returned if detection is
/// inconclusive.
pub fn setup_terminal() -> io::Result<(Terminal<CrosstermBackend<std::io::Stdout>>, caps::TermCaps)>
{
    let capabilities = caps::detect_capabilities_from_env();

    crossterm::terminal::enable_raw_mode()?;
    let mut stdout = std::io::stdout();
    // Grok-style init: mouse capture (kept for wheel scrolling) + focus +
    // bracketed paste + sync update + cursor
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
    writer.queue(PushKeyboardEnhancementFlags(
        KeyboardEnhancementFlags::DISAMBIGUATE_ESCAPE_CODES
            | KeyboardEnhancementFlags::REPORT_ALL_KEYS_AS_ESCAPE_CODES
            | KeyboardEnhancementFlags::REPORT_EVENT_TYPES
            | KeyboardEnhancementFlags::REPORT_ALTERNATE_KEYS,
    ))?;
    writer.queue(PushKeyboardEnhancementFlags(
        KeyboardEnhancementFlags::DISAMBIGUATE_ESCAPE_CODES,
    ))?;
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
    writer.queue(PopKeyboardEnhancementFlags)?;
    reset_xterm_modify_other_keys(writer)
}

pub fn restore_terminal_graphics<W: io::Write>(
    writer: &mut W,
    capabilities: caps::TermCaps,
) -> io::Result<()> {
    enable_mouse_grok_style(writer, &capabilities)?;
    let _ = push_keyboard_enhancement_flags(writer);
    Ok(())
}

/// Write the mouse mode enable escape sequence for the given capability level.
pub fn enable_mouse<W: io::Write>(writer: &mut W, caps: caps::MouseCapability) -> io::Result<()> {
    match caps {
        caps::MouseCapability::None => Ok(()),
        caps::MouseCapability::Legacy
        | caps::MouseCapability::Sgr
        | caps::MouseCapability::SgrExtended => {
            writer.queue(EnableMouseCapture)?;
            writer.flush()
        }
    }
}

fn enable_focus_tracking<W: io::Write>(writer: &mut W, caps: &caps::TermCaps) -> io::Result<()> {
    if caps.focus_tracking {
        writer.queue(EnableFocusChange)?;
    }
    Ok(())
}

fn enable_bracketed_paste<W: io::Write>(writer: &mut W) -> io::Result<()> {
    writer.queue(EnableBracketedPaste)?;
    Ok(())
}

fn hide_cursor_and_set_block<W: io::Write>(writer: &mut W) -> io::Result<()> {
    writer.queue(Hide)?;
    writer.queue(SetCursorStyle::BlinkingBlock)?;
    Ok(())
}

/// Write the Grok-style mouse + terminal init sequence.
/// Mouse capture is enabled (gated on `caps.mouse != None`) solely so the
/// terminal reports wheel-scroll events; runie does not handle click, drag,
/// or move. Focus tracking, bracketed paste, and cursor setup are emitted
/// unconditionally (unsupported terminals ignore them). Synchronized updates
/// are NOT enabled here: they are bracketed per frame by the render loop (see
/// `begin_frame_sync`/`end_frame_sync`), because a session-long BSU makes
/// 2026-aware terminals buffer grid updates indefinitely.
pub fn enable_mouse_grok_style<W: io::Write>(
    writer: &mut W,
    caps: &caps::TermCaps,
) -> io::Result<()> {
    writer.queue(EnterAlternateScreen)?;
    if caps.mouse != caps::MouseCapability::None {
        writer.queue(EnableMouseCapture)?;
    }
    enable_focus_tracking(writer, caps)?;
    enable_bracketed_paste(writer)?;
    hide_cursor_and_set_block(writer)?;
    writer.flush()
}

fn end_sync_update<W: io::Write>(writer: &mut W) -> io::Result<()> {
    writer.write_all(b"\x1b[?2026l")?;
    Ok(())
}

fn show_cursor<W: io::Write>(writer: &mut W) -> io::Result<()> {
    writer.queue(Show)?;
    Ok(())
}

fn disable_bracketed_paste<W: io::Write>(writer: &mut W) -> io::Result<()> {
    writer.queue(DisableBracketedPaste)?;
    Ok(())
}

fn disable_focus_tracking<W: io::Write>(writer: &mut W) -> io::Result<()> {
    writer.queue(DisableFocusChange)?;
    Ok(())
}

fn disable_all_mouse_modes<W: io::Write>(writer: &mut W) -> io::Result<()> {
    writer.queue(DisableMouseCapture)?;
    Ok(())
}

fn leave_alternate_screen<W: io::Write>(writer: &mut W) -> io::Result<()> {
    writer.queue(LeaveAlternateScreen)?;
    Ok(())
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
    fn enable_mouse_legacy_enables_mouse_capture() {
        let mut buf = Vec::new();
        enable_mouse(&mut buf, caps::MouseCapability::Legacy).unwrap();
        let s = String::from_utf8(buf).unwrap();
        // EnableMouseCapture enables all grok mouse modes (1000, 1002, 1003, 1015, 1006).
        assert!(s.contains("?1000h"), "missing ?1000h: {s}");
        assert!(s.contains("?1002h"), "missing ?1002h: {s}");
        assert!(s.contains("?1003h"), "missing ?1003h: {s}");
    }

    #[test]
    fn enable_mouse_sgr_also_enables_mouse_capture() {
        let mut buf = Vec::new();
        enable_mouse(&mut buf, caps::MouseCapability::Sgr).unwrap();
        let s = String::from_utf8(buf).unwrap();
        // EnableMouseCapture enables all grok mouse modes.
        assert!(s.contains("?1000h"), "missing ?1000h: {s}");
    }

    #[test]
    fn enable_mouse_sgr_extended_also_enables_mouse_capture() {
        let mut buf = Vec::new();
        enable_mouse(&mut buf, caps::MouseCapability::SgrExtended).unwrap();
        let s = String::from_utf8(buf).unwrap();
        // EnableMouseCapture enables all grok mouse modes.
        assert!(s.contains("?1000h"), "missing ?1000h: {s}");
    }

    // ── Grok-style init tests ──────────────────────────────────────────────

    #[test]
    fn mouse_init_sequence_includes_all_grok_modes() {
        let mut buf = Vec::new();
        let caps = caps::TermCaps {
            mouse: caps::MouseCapability::Sgr,
            focus_tracking: true,
            ..Default::default()
        };
        enable_mouse_grok_style(&mut buf, &caps).unwrap();
        let s = String::from_utf8(buf).unwrap();
        assert!(
            s.contains("\x1b[?1049h"),
            "missing ?1049h (alternate screen)"
        );
        // EnableMouseCapture enables 1000, 1002, 1003, 1015, 1006.
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
        // Synchronized updates MUST NOT be enabled for the whole session:
        // holding BSU across frames makes 2026-aware terminals (tmux >= 3.2)
        // buffer grid updates indefinitely, so small diffs (short feeds)
        // never reach the screen. Sync is bracketed PER FRAME instead.
        assert!(
            !s.contains("\x1b[?2026h"),
            "startup must not enable sync updates for the whole session"
        );
        assert!(s.contains("\x1b[?25l"), "missing ?25l (hide cursor)");
        assert!(s.contains("\x1b[1 q"), "missing block cursor");
    }

    #[test]
    fn init_and_runtime_emit_no_sync_markers() {
        // 2026 synchronized updates are DISABLED entirely: tmux 3.7b (and
        // other 2026-aware terminals) buffer grid updates while BSU is
        // active, and runie renders continuously — buffered frames lose
        // small diffs, so short feeds render blank. ratatui's own
        // diff-based flush already prevents tearing without 2026.
        let mut buf = Vec::new();
        let caps = caps::TermCaps::default();
        enable_mouse_grok_style(&mut buf, &caps).unwrap();
        let s = String::from_utf8(buf).unwrap();
        assert!(
            !s.contains("\x1b[?2026"),
            "init must not touch sync-update mode: {s:?}"
        );
    }

    #[test]
    fn mouse_init_omits_mouse_when_capability_is_none() {
        let mut buf = Vec::new();
        let caps = caps::TermCaps {
            mouse: caps::MouseCapability::None,
            focus_tracking: false,
            ..Default::default()
        };
        enable_mouse_grok_style(&mut buf, &caps).unwrap();
        let s = String::from_utf8(buf).unwrap();
        // Only alternate screen + focus (false) + bracketed + sync + cursor (always emitted)
        assert!(
            s.contains("\x1b[?1049h"),
            "should still have alternate screen"
        );
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
        assert!(s.contains("\x1b[?1000l"), "missing ?1000l");
        assert!(s.contains("\x1b[?1049l"), "missing ?1049l (exit alternate)");
    }
}
