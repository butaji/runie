//! Reducer tests for state updates.

use crate::tui::state::{AppState, AnimationState, CommandPaletteState, Msg, Cmd, ScrollState, ContextState, PermissionModalState, PendingPermission, TuiMode, ClearInputConfirm, TopBarState};
use crate::components::{MessageItem, SessionTreeNavigator};
use crate::components::CommandPalette;
use crate::tui::update::update;
use runie_agent::{AgentEvent, AgentMessage, PermissionDecision};
use runie_ai::TokenUsage as AiTokenUsage;
use runie_agent::TokenUsage as AgentTokenUsage;
use ratatui_textarea::{TextArea, Input, Key};

pub fn make_state() -> AppState {
    let mut state = AppState::default();
    state.mode = TuiMode::Chat;
    state
}

pub fn make_state_with_text(text: &str) -> AppState {
    let mut s = make_state();
    s.textarea = TextArea::new(vec![text.to_string()]);
    s.current_model = Some("gpt-4".to_string());
    s
}

pub fn type_char(state: &mut AppState, c: char) {
    state.textarea.input(Input { key: Key::Char(c), ctrl: false, alt: false, shift: false });
}

pub fn type_enter(state: &mut AppState) {
    state.textarea.input(Input { key: Key::Enter, ctrl: false, alt: false, shift: false });
}

mod input_tests;
mod agent_tests;
mod permission_tests;
mod scroll_tests;
mod clear_tests;
mod submit_tests;

pub use input_tests::*;
pub use agent_tests::*;
pub use permission_tests::*;
pub use scroll_tests::*;
pub use clear_tests::*;
pub use submit_tests::*;
