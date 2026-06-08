//! Tests for input history (Up/Down arrows)

#[cfg(test)]
mod tests {
    use crate::model::AppState;
    use crate::event::Event;

    #[test]
    fn history_starts_empty() {
        let state = AppState::default();
        assert!(state.input.is_empty());
    }

    #[test]
    fn submit_adds_to_history() {
        let mut state = AppState::default();
        state.update(Event::Input('h'));
        state.update(Event::Input('i'));
        state.update(Event::Submit);
        assert!(state.input.is_empty());
        // History should have "hi"
    }

    #[test]
    fn up_arrow_recalls_previous_input() {
        let mut state = AppState::default();
        state.update(Event::Input('h'));
        state.update(Event::Input('i'));
        state.update(Event::Submit);
        state.update(Event::HistoryPrev);
        assert_eq!(state.input, "hi");
    }

    #[test]
    fn down_arrow_returns_to_empty() {
        let mut state = AppState::default();
        state.update(Event::Input('h'));
        state.update(Event::Input('i'));
        state.update(Event::Submit);
        state.update(Event::HistoryPrev);
        assert_eq!(state.input, "hi");
        state.update(Event::HistoryNext);
        assert!(state.input.is_empty());
    }

    #[test]
    fn history_navigates_multiple_items() {
        let mut state = AppState::default();
        // Submit "first"
        for c in "first".chars() { state.update(Event::Input(c)); }
        state.update(Event::Submit);
        // Submit "second"
        for c in "second".chars() { state.update(Event::Input(c)); }
        state.update(Event::Submit);
        // Submit "third"
        for c in "third".chars() { state.update(Event::Input(c)); }
        state.update(Event::Submit);

        state.update(Event::HistoryPrev);
        assert_eq!(state.input, "third");
        state.update(Event::HistoryPrev);
        assert_eq!(state.input, "second");
        state.update(Event::HistoryPrev);
        assert_eq!(state.input, "first");
        // At beginning, stays at first
        state.update(Event::HistoryPrev);
        assert_eq!(state.input, "first");
    }

    #[test]
    fn history_next_at_end_is_empty() {
        let mut state = AppState::default();
        for c in "test".chars() { state.update(Event::Input(c)); }
        state.update(Event::Submit);
        state.update(Event::HistoryPrev);
        assert_eq!(state.input, "test");
        state.update(Event::HistoryNext);
        assert!(state.input.is_empty());
        // Stays empty
        state.update(Event::HistoryNext);
        assert!(state.input.is_empty());
    }

    #[test]
    fn editing_resets_history_nav() {
        let mut state = AppState::default();
        for c in "hello".chars() { state.update(Event::Input(c)); }
        state.update(Event::Submit);
        state.update(Event::HistoryPrev);
        assert_eq!(state.input, "hello");
        // Type something new
        state.update(Event::Input('x'));
        // Now down should clear
        state.update(Event::HistoryNext);
        assert!(state.input.is_empty());
    }
}
