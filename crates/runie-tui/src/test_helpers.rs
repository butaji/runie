//! Consolidated test helpers for runie-tui.
//!
//! Single source of truth for all test helper functions.
//! Re-exports from this module in tests instead of duplicating helpers.

#[cfg(test)]
pub mod helpers {
    use crate::tui::state::{AppState, TuiMode, Msg, Cmd, Onboarding};
    use crate::components::MessageItem;
    use crate::tui::update::update;
    use std::sync::Arc;

    /// Create default AppState in Chat mode.
    pub fn make_state() -> AppState {
        AppState::default()
    }

    /// Create AppState with text in textarea.
    pub fn make_state_with_text(text: &str) -> AppState {
        let mut state = make_state();
        state.current_model = Some("gpt-4".to_string()); // P0-2 FIX: Set model for submit tests
        state.textarea.insert_str(text);
        state
    }

    /// Create AppState in Onboarding mode.
    pub fn make_onboarding_state() -> AppState {
        let mut state = make_state();
        state.transition_to(TuiMode::Onboarding, "test").unwrap();
        state.onboarding = Some(Onboarding::new());
        state
    }

    /// Create AppState with messages.
    pub fn make_state_with_messages(msgs: Vec<MessageItem>) -> AppState {
        let mut state = make_state();
        state.messages = Arc::from(msgs);
        state
    }

    /// Create AppState in Chat mode with input text.
    pub fn make_chat_state_with_input(text: &str) -> AppState {
        let mut state = make_state_with_text(text);
        state.current_model = Some("gpt-4".to_string());
        state
    }

    /// Create AppState in CommandPalette mode with open palette.
    pub fn make_palette_state() -> AppState {
        let mut state = make_state();
        state.transition_to(TuiMode::CommandPalette, "test").unwrap();
        state.command_palette.open = true;
        state
    }

    /// Create AppState with modal open in specified mode.
    pub fn make_state_with_modal(mode: TuiMode) -> AppState {
        let mut state = make_state();
        state.transition_to(mode.clone(), "test").unwrap();
        if mode == TuiMode::CommandPalette {
            state.command_palette.open = true;
        }
        state
    }

    /// MockTui for testing dirty flag pattern.
    ///
    /// Mirrors the exact structure of Tui.update() behavior:
    /// - Sets dirty=true BEFORE calling the reducer
    /// - Then calls the free function to update state
    pub struct MockTui {
        pub state: AppState,
    }

    impl MockTui {
        /// Create new MockTui with dirty from state.
        pub fn new() -> Self {
            Self {
                state: AppState::default(),
            }
        }

        /// Update state, setting dirty flag BEFORE calling reducer.
        /// Returns the commands produced by update.
        pub fn update(&mut self, msg: Msg) -> Vec<Cmd> {
            self.state.mark_dirty();
            update(&mut self.state, msg)
        }

        /// Check if dirty flag is set.
        pub fn is_dirty(&self) -> bool {
            self.state.is_dirty()
        }

        /// Clear the dirty flag.
        pub fn clear_dirty(&mut self) {
            self.state.clear_dirty();
        }

        /// Simulate render, returning true if it would actually render.
        /// Clears dirty flag after render.
        pub fn render(&mut self) -> bool {
            if !self.state.is_dirty() {
                return false;
            }
            self.state.clear_dirty();
            true
        }
    }

}
