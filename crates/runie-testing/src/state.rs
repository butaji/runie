//! Shared test helpers for AppState manipulation.

use runie_core::event::InputEvent;
use runie_core::model::AppState;

/// Returns a fresh `AppState` with default values.
pub fn fresh_state() -> AppState {
    AppState::default()
}

/// Simulates typing `text` into the input buffer of `state`.
pub fn type_str(state: &mut AppState, text: &str) {
    for c in text.chars() {
        state.update(InputEvent::Input(c));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shared_type_str_appends() {
        let mut state = fresh_state();
        assert_eq!(state.input.input, "");
        type_str(&mut state, "hello");
        assert_eq!(state.input.input, "hello");
        type_str(&mut state, " world");
        assert_eq!(state.input.input, "hello world");
    }
}
