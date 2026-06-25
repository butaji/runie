//! Shared test helpers for AppState manipulation.
//!
//! Canonical source for `fresh_state()`, `type_str()`, and `exec()` shared by
//! `runie-core` and `runie-tui` tests.  Crates import from `runie_testing`
/// instead of defining their own copies.
use runie_core::event::Event;
use runie_core::model::AppState;

/// Returns a fresh `AppState` with default values.
pub fn fresh_state() -> AppState {
    AppState::default()
}

/// Simulates typing `text` into the input buffer of `state`.
pub fn type_str(state: &mut AppState, text: &str) {
    for c in text.chars() {
        state.update(Event::Input(c));
    }
}

/// Set input buffer directly and submit — bypasses the command palette.
/// Use for slash commands that need arguments.
pub fn exec(state: &mut AppState, text: &str) {
    *state.input_mut().input_mut() = text.into();
    *state.input_mut().cursor_pos_mut() = text.len();
    state.update(Event::Submit);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shared_type_str_appends() {
        let mut state = fresh_state();
        assert_eq!(state.input().input, "");
        type_str(&mut state, "hello");
        assert_eq!(state.input().input, "hello");
        type_str(&mut state, " world");
        assert_eq!(state.input().input, "hello world");
    }

    #[test]
    fn shared_exec_submits_command() {
        let mut state = fresh_state();
        assert_eq!(state.input().input, "");
        exec(&mut state, "/save");
        assert!(state.input().cursor_pos >= 0);
    }
}
