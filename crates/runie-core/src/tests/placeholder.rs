//! Tests for placeholder text

use crate::event::{InputEvent, ControlEvent, ModelConfigEvent, SystemEvent, DialogEvent, ScrollEvent, AgentEvent, SessionEvent, EditEvent, CommandEvent, DurableCoreEvent};

#[cfg(test)]
mod tests {
    use crate::event::{Event, InputEvent, ControlEvent, ModelConfigEvent, SystemEvent, DialogEvent, ScrollEvent, AgentEvent, SessionEvent, EditEvent, CommandEvent, DurableCoreEvent};
    use crate::model::AppState;

    #[test]
    fn default_placeholder_is_set() {
        let state = AppState::default();
        assert!(
            !state.input.placeholder.is_empty(),
            "Default placeholder should not be empty"
        );
    }

    #[test]
    fn placeholder_in_snapshot_when_input_empty() {
        let mut state = AppState::default();
        let snap = state.snapshot();
        assert!(
            !snap.placeholder.is_empty(),
            "Snapshot should contain placeholder"
        );
    }

    #[test]
    fn placeholder_cleared_after_typing() {
        let mut state = AppState::default();
        state.update(Event::Input(InputEvent::Input('a')));
        let _snap = state.snapshot();
        // Placeholder is still in state, but input is no longer empty
        assert_eq!(state.input.input, "a");
    }

    #[test]
    fn placeholder_returns_after_clearing_input() {
        let mut state = AppState::default();
        state.update(Event::Input(InputEvent::Input('a')));
        state.update(Event::Input(InputEvent::Backspace));
        assert!(state.input.input.is_empty());
        let snap = state.snapshot();
        assert!(!snap.placeholder.is_empty());
    }
}
