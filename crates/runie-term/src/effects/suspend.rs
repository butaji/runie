//! Suspend (SIGTSTP) effect handler.

use crate::terminal::caps::TerminalCapabilities;
use crate::terminal_setup;
use runie_core::{AppState, Snapshot};
use tokio::sync::watch;

/// Restore the terminal, suspend the process, then resume and redraw.
pub fn run(caps: TerminalCapabilities, render_tx: watch::Sender<Snapshot>, state: &mut AppState) {
    #[cfg(unix)]
    {
        let _ = terminal_setup::reset_keyboard_enhancements(&mut std::io::stdout());
        let _ = crossterm::execute!(std::io::stdout(), crossterm::terminal::LeaveAlternateScreen,);
        let _ = crossterm::terminal::disable_raw_mode();

        let _ = nix::sys::signal::kill(nix::unistd::Pid::this(), nix::sys::signal::Signal::SIGTSTP);

        let _ = crossterm::terminal::enable_raw_mode();
        let _ = terminal_setup::restore_terminal_graphics(&mut std::io::stdout(), caps);

        state.ensure_fresh();
        let _ = render_tx.send(state.snapshot());
    }

    // Suspend is a no-op on non-Unix platforms.
    let _ = (caps, render_tx, state);
}
