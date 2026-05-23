//! Helper functions for hotkey tests.

#[allow(clippy::unwrap_used)]
pub fn simulate_key(code: crossterm::event::KeyCode, modifiers: crossterm::event::KeyModifiers, mode: crate::tui::state::TuiMode) -> Option<crate::tui::state::Msg> {
    use crossterm::event::{Event, KeyEvent, KeyEventKind, KeyEventState};
    let event = Event::Key(KeyEvent {
        code,
        modifiers,
        kind: KeyEventKind::Press,
        state: KeyEventState::NONE,
    });
    let state = crate::tui::state::AppState {
        mode,
        ..Default::default()
    };
    crate::tui::events::event_to_msg(event, &state)
}

/// Helper: create AppState in Chat mode with some input typed
pub fn make_chat_state_with_input(text: &str) -> crate::tui::state::AppState {
    use crate::tui::state::AppState;
    use crate::tui::state::TuiMode;
    use crate::tui::update::update;
    use crate::tui::state::Msg;

    let mut state = AppState {
        mode: TuiMode::Chat,
        ..Default::default()
    };
    for c in text.chars() {
        update(&mut state, Msg::InsertChar(c));
    }
    state
}

/// Helper: create AppState in CommandPalette mode
#[allow(dead_code)]
pub fn make_palette_state() -> crate::tui::state::AppState {
    use crate::tui::state::AppState;
    use crate::tui::state::TuiMode;

    let mut state = AppState::default();
    state.mode = TuiMode::CommandPalette;
    state.command_palette.open = true;
    state
}

/// Helper: create AppState with modal open
pub fn make_state_with_modal(mode: crate::tui::state::TuiMode) -> crate::tui::state::AppState {
    use crate::tui::state::AppState;
    use crate::tui::state::TuiMode;

    let mut state = AppState {
        mode: mode.clone(),
        ..Default::default()
    };
    if mode == TuiMode::CommandPalette {
        state.command_palette.open = true;
    }
    state
}
