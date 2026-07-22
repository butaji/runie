//! Terminal setup and progressive keyboard enhancement helpers.
//!
//! Uses `crossterm` commands for standard terminal sequences.
//! Non-standard sequences (xterm modifyOtherKeys, synchronized update)
//! are kept as raw bytes.
//!
//! ## Mouse policy
//!
//! Runie NEVER enables mouse capture. Any mouse-reporting mode makes the
//! terminal deliver mouse events to the app instead of performing native
//! text selection, so capture would force users to hold a
//! terminal-specific modifier (Shift/Option/Fn) to copy feed text.
//! Leaving the mouse to the terminal gives plain click-drag selection;
//! feed scrolling is keyboard-driven (PgUp/PgDn and Esc nav mode; ↑/↓ with
//! an empty input box recall prompt history — grok parity).

use crate::terminal::caps;
use crossterm::{
    cursor::{Hide, SetCursorStyle, Show},
    event::{
        DisableBracketedPaste, DisableFocusChange, DisableMouseCapture, EnableBracketedPaste, EnableFocusChange,
        KeyboardEnhancementFlags, PopKeyboardEnhancementFlags, PushKeyboardEnhancementFlags,
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
pub fn setup_terminal() -> io::Result<(Terminal<CrosstermBackend<std::io::Stdout>>, caps::TermCaps)> {
    let capabilities = caps::detect_capabilities_from_env();

    crossterm::terminal::enable_raw_mode()?;
    let mut stdout = std::io::stdout();
    // Alternate screen + focus + bracketed paste + cursor. Mouse capture is
    // deliberately NOT enabled (native terminal selection, see module docs).
    enter_tui_mode(&mut stdout, &capabilities)?;
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

pub fn restore_terminal_graphics<W: io::Write>(writer: &mut W, capabilities: caps::TermCaps) -> io::Result<()> {
    enter_tui_mode(writer, &capabilities)?;
    let _ = push_keyboard_enhancement_flags(writer);
    Ok(())
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

/// Write the TUI-mode init sequence: alternate screen, focus tracking,
/// bracketed paste, cursor setup. Mouse capture is intentionally absent —
/// see the module-level mouse policy. Synchronized updates are NOT enabled
/// here: they are bracketed per frame by the render loop (see
/// `begin_frame_sync`/`end_frame_sync`), because a session-long BSU makes
/// 2026-aware terminals buffer grid updates indefinitely.
pub fn enter_tui_mode<W: io::Write>(writer: &mut W, caps: &caps::TermCaps) -> io::Result<()> {
    writer.queue(EnterAlternateScreen)?;
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
    // Defensive: runie never enables mouse capture, but releasing it on exit
    // restores native selection if a crash or an older version left it on.
    writer.queue(DisableMouseCapture)?;
    Ok(())
}

fn leave_alternate_screen<W: io::Write>(writer: &mut W) -> io::Result<()> {
    writer.queue(LeaveAlternateScreen)?;
    Ok(())
}

/// Disable all TUI terminal modes.
pub fn leave_tui_mode<W: io::Write>(writer: &mut W) -> io::Result<()> {
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

    // ── TUI-mode init tests ──────────────────────────────────────────────

    #[test]
    fn init_sequence_enables_no_mouse_capture() {
        // Mouse capture is NEVER enabled — any mouse-reporting mode makes the
        // terminal send mouse events to the app instead of performing native
        // text selection. Runie leaves the mouse to the terminal so users can
        // click-drag to select and copy feed text. Feed scrolling is
        // keyboard-driven (PgUp/PgDn, nav mode, ↑/↓ on an empty input — also
        // what mouse wheels send in alternate-scroll terminals).
        for mouse in [
            caps::MouseCapability::None,
            caps::MouseCapability::Legacy,
            caps::MouseCapability::Sgr,
            caps::MouseCapability::SgrExtended,
        ] {
            let mut buf = Vec::new();
            let caps = caps::TermCaps { mouse, focus_tracking: true, ..Default::default() };
            enter_tui_mode(&mut buf, &caps).unwrap();
            let s = String::from_utf8(buf).unwrap();
            for mode in ["?1000h", "?1002h", "?1003h", "?1006h", "?1015h"] {
                assert!(
                    !s.contains(mode),
                    "mouse capture mode {mode} must not be emitted (caps {mouse:?}): {s:?}"
                );
            }
        }
    }

    #[test]
    fn init_sequence_keeps_terminal_modes() {
        let mut buf = Vec::new();
        let caps = caps::TermCaps { mouse: caps::MouseCapability::Sgr, focus_tracking: true, ..Default::default() };
        enter_tui_mode(&mut buf, &caps).unwrap();
        let s = String::from_utf8(buf).unwrap();
        assert!(
            s.contains("\x1b[?1049h"),
            "missing ?1049h (alternate screen)"
        );
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
        enter_tui_mode(&mut buf, &caps).unwrap();
        let s = String::from_utf8(buf).unwrap();
        assert!(
            !s.contains("\x1b[?2026"),
            "init must not touch sync-update mode: {s:?}"
        );
    }

    #[test]
    fn cleanup_sequence_disables_all_modes() {
        let mut buf = Vec::new();
        leave_tui_mode(&mut buf).unwrap();
        let s = String::from_utf8(buf).unwrap();
        assert!(s.contains("\x1b[?2026l"), "missing ?2026l (sync end)");
        assert!(s.contains("\x1b[?25h"), "missing ?25h (show cursor)");
        assert!(
            s.contains("\x1b[?2004l"),
            "missing ?2004l (bracketed paste)"
        );
        assert!(s.contains("\x1b[?1004l"), "missing ?1004l (focus)");
        // Defensive: always release mouse capture on exit so a terminal left
        // captured by a crash (or an older runie version) gets native
        // selection back.
        assert!(s.contains("\x1b[?1000l"), "missing ?1000l");
        assert!(s.contains("\x1b[?1049l"), "missing ?1049l (exit alternate)");
    }
}
