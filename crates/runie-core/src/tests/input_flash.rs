//! Tests for input validation flash

use crate::event::{InputEvent, ControlEvent, ModelConfigEvent, SystemEvent, DialogEvent, ScrollEvent, AgentEvent, SessionEvent, EditEvent, CommandEvent, DurableCoreEvent};

#[cfg(test)]
mod tests {
    use crate::event::{Event, InputEvent, ControlEvent, ModelConfigEvent, SystemEvent, DialogEvent, ScrollEvent, AgentEvent, SessionEvent, EditEvent, CommandEvent, DurableCoreEvent};
    use crate::model::AppState;

    #[test]
    fn flash_increments_on_noop() {
        let mut state = AppState::default();
        assert_eq!(state.input.input_flash, 0);
        // Cursor left at position 0 is a no-op
        state.update(Event::Input(InputEvent::CursorLeft));
        assert!(
            state.input.input_flash > 0,
            "Flash should trigger on no-op cursor move"
        );
    }

    #[test]
    fn flash_does_not_trigger_on_valid_action() {
        let mut state = AppState::default();
        state.update(Event::Input(InputEvent::Input('a')));
        state.update(Event::Input(InputEvent::CursorLeft));
        assert_eq!(
            state.input.input_flash, 0,
            "Flash should not trigger on valid cursor move"
        );
    }

    #[test]
    fn flash_does_not_trigger_on_typing() {
        let mut state = AppState::default();
        state.update(Event::Input(InputEvent::Input('a')));
        assert_eq!(state.input.input_flash, 0);
    }

    #[test]
    fn flash_on_scroll_up_when_empty() {
        let mut state = AppState::default();
        state.update(Event::Scroll(ScrollEvent::Up));
        assert!(
            state.input.input_flash > 0,
            "Flash should trigger when scrolling with no content"
        );
    }

    #[test]
    fn flash_count_is_limited() {
        let mut state = AppState::default();
        state.update(Event::Input(InputEvent::CursorLeft));
        let flash1 = state.input.input_flash;
        state.update(Event::Input(InputEvent::CursorLeft));
        assert_eq!(
            state.input.input_flash, flash1,
            "Flash should not accumulate beyond max"
        );
    }
}
