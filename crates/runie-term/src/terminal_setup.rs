//! Terminal setup and progressive keyboard enhancement helpers.

use crossterm::event::{KeyboardEnhancementFlags, PushKeyboardEnhancementFlags};
use ratatui::{backend::CrosstermBackend, Terminal};
use std::io;

pub fn setup_terminal() -> io::Result<Terminal<CrosstermBackend<std::io::Stdout>>> {
    crossterm::terminal::enable_raw_mode()?;
    let mut stdout = std::io::stdout();
    crossterm::execute!(
        &mut stdout,
        crossterm::terminal::EnterAlternateScreen,
        crossterm::event::EnableBracketedPaste,
    )?;
    // Progressive enhancement: modern terminals (kitty keyboard protocol)
    // report modified keys and disambiguate escape sequences. The legacy
    // Windows console API does not support this, so ignore failure.
    let _ = push_keyboard_enhancement_flags(&mut stdout);
    Terminal::new(CrosstermBackend::new(stdout))
}

pub fn push_keyboard_enhancement_flags<W: io::Write>(writer: &mut W) -> io::Result<()> {
    crossterm::execute!(
        writer,
        PushKeyboardEnhancementFlags(
            KeyboardEnhancementFlags::DISAMBIGUATE_ESCAPE_CODES
                | KeyboardEnhancementFlags::REPORT_ALL_KEYS_AS_ESCAPE_CODES
        ),
    )
}

pub fn restore_terminal_graphics<W: io::Write>(writer: &mut W) -> io::Result<()> {
    crossterm::execute!(
        writer,
        crossterm::terminal::EnterAlternateScreen,
        crossterm::event::EnableBracketedPaste,
    )?;
    let _ = push_keyboard_enhancement_flags(writer);
    Ok(())
}
