//! Tests for input history (Up/Down arrows)

use crate::event::{InputEvent, ControlEvent, ModelConfigEvent, SystemEvent, DialogEvent, ScrollEvent, AgentEvent, SessionEvent, EditEvent, CommandEvent, DurableCoreEvent};

#[cfg(test)]
mod tests {
    use crate::event::{Event, InputEvent, ControlEvent, ModelConfigEvent, SystemEvent, DialogEvent, ScrollEvent, AgentEvent, SessionEvent, EditEvent, CommandEvent, DurableCoreEvent};
    use crate::model::AppState;

    #[test]
    fn history_starts_empty() {
        let state = AppState::default();
        assert!(state.input.input.is_empty());
    }

    #[test]
    fn submit_adds_to_history() {
        let mut state = AppState::default();
        state.update(Event::Input(InputEvent::Input('h')));
        state.update(Event::Input(InputEvent::Input('i')));
        state.update(Event::submit());
        assert!(state.input.input.is_empty());
        // History should have "hi"
    }

    #[test]
    fn up_arrow_recalls_previous_input() {
        let mut state = AppState::default();
        state.update(Event::Input(InputEvent::Input('h')));
        state.update(Event::Input(InputEvent::Input('i')));
        state.update(Event::submit());
        state.update(Event::Input(InputEvent::HistoryPrev));
        assert_eq!(state.input.input, "hi");
    }

    #[test]
    fn down_arrow_returns_to_empty() {
        let mut state = AppState::default();
        state.update(Event::Input(InputEvent::Input('h')));
        state.update(Event::Input(InputEvent::Input('i')));
        state.update(Event::submit());
        state.update(Event::Input(InputEvent::HistoryPrev));
        assert_eq!(state.input.input, "hi");
        state.update(Event::Input(InputEvent::HistoryNext));
        assert!(state.input.input.is_empty());
    }

    #[test]
    fn history_navigates_multiple_items() {
        let mut state = AppState::default();
        // Submit "first"
        for c in "first".chars() {
            state.update(Event::Input(InputEvent::Input(c)));
        }
        state.update(Event::submit());
        // Submit "second"
        for c in "second".chars() {
            state.update(Event::Input(InputEvent::Input(c)));
        }
        state.update(Event::submit());
        // Submit "third"
        for c in "third".chars() {
            state.update(Event::Input(InputEvent::Input(c)));
        }
        state.update(Event::submit());

        state.update(Event::Input(InputEvent::HistoryPrev));
        assert_eq!(state.input.input, "third");
        state.update(Event::Input(InputEvent::HistoryPrev));
        assert_eq!(state.input.input, "second");
        state.update(Event::Input(InputEvent::HistoryPrev));
        assert_eq!(state.input.input, "first");
        // At beginning, stays at first
        state.update(Event::Input(InputEvent::HistoryPrev));
        assert_eq!(state.input.input, "first");
    }

    #[test]
    fn history_next_at_end_is_empty() {
        let mut state = AppState::default();
        for c in "test".chars() {
            state.update(Event::Input(InputEvent::Input(c)));
        }
        state.update(Event::submit());
        state.update(Event::Input(InputEvent::HistoryPrev));
        assert_eq!(state.input.input, "test");
        state.update(Event::Input(InputEvent::HistoryNext));
        assert!(state.input.input.is_empty());
        // Stays empty
        state.update(Event::Input(InputEvent::HistoryNext));
        assert!(state.input.input.is_empty());
    }

    #[test]
    fn editing_resets_history_nav() {
        let mut state = AppState::default();
        for c in "hello".chars() {
            state.update(Event::Input(InputEvent::Input(c)));
        }
        state.update(Event::submit());
        state.update(Event::Input(InputEvent::HistoryPrev));
        assert_eq!(state.input.input, "hello");
        // Type something new
        state.update(Event::Input(InputEvent::Input('x')));
        // Now down should clear
        state.update(Event::Input(InputEvent::HistoryNext));
        assert!(state.input.input.is_empty());
    }
}
