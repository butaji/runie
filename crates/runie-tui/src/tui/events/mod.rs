//! Event handling - converts crossterm events to app messages.

mod keyboard;
mod mouse;

use crossterm::event::Event;
use crate::tui::state::{AppState, Msg};
pub use keyboard::key_to_msg;

// --- Key classification helpers ---

fn key_event_to_msg(key: crossterm::event::KeyEvent, state: &AppState) -> Vec<Msg> {
    key_to_msg(key, state).map_or_else(Vec::new, |m| vec![m])
}

pub fn event_to_msg(event: Event, state: &AppState) -> Vec<Msg> {
    match event {
        Event::Key(key) => key_event_to_msg(key, state),
        Event::Paste(text) => {
            if matches!(state.mode, crate::tui::state::TuiMode::Permission | crate::tui::state::TuiMode::Overlay) {
                vec![]
            } else {
                vec![crate::tui::state::Msg::Paste(text)]
            }
        }
        Event::Resize(w, h) => vec![crate::tui::state::Msg::Resize(w, h)],
        Event::Mouse(mouse_event) => mouse::mouse_event_to_msg(mouse_event),
        _ => Vec::new(),
    }
}
