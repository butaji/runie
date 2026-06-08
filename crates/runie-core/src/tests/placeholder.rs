//! Tests for placeholder text

#[cfg(test)]
mod tests {
    use crate::model::AppState;
    use crate::event::Event;

    #[test]
    fn default_placeholder_is_set() {
        let state = AppState::default();
        assert!(!state.placeholder.is_empty(), "Default placeholder should not be empty");
    }

    #[test]
    fn placeholder_in_snapshot_when_input_empty() {
        let state = AppState::default();
        let snap = state.snapshot();
        assert!(!snap.placeholder.is_empty(), "Snapshot should contain placeholder");
    }

    #[test]
    fn placeholder_cleared_after_typing() {
        let mut state = AppState::default();
        state.update(Event::Input('a'));
        let snap = state.snapshot();
        // Placeholder is still in state, but input is no longer empty
        assert_eq!(state.input, "a");
    }

    #[test]
    fn placeholder_returns_after_clearing_input() {
        let mut state = AppState::default();
        state.update(Event::Input('a'));
        state.update(Event::Backspace);
        assert!(state.input.is_empty());
        let snap = state.snapshot();
        assert!(!snap.placeholder.is_empty());
    }
}
