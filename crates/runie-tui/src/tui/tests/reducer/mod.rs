//! Reducer tests for state updates.

use crate::tui::state::{AppState, AnimationState, CommandPaletteState, Msg, Cmd, ScrollState, TopBarState, PermissionModalState, PendingPermission, TuiMode, ClearInputConfirm};
use crate::components::{MessageItem, SessionTreeNavigator};
use crate::components::CommandPalette;
use crate::tui::update::update;
use runie_agent::{AgentEvent, AgentMessage, PermissionDecision};
use runie_ai::TokenUsage as AiTokenUsage;
use runie_agent::TokenUsage as AgentTokenUsage;
use ratatui_textarea::{TextArea, Input, Key};

pub fn make_state() -> AppState {
    AppState {
        messages: vec![],
        textarea: TextArea::default(),
        input_right_info: String::new(),
        mode: TuiMode::Chat,
        running: true,
        show_sidebar: false,
        agent_running: false,
        current_model: None,
        top_bar: TopBarState::default(),
        permission_modal: PermissionModalState::default(),
        command_palette: CommandPaletteState::default(),
        scroll: ScrollState::default(),
        animation: AnimationState::default(),
        diff_viewer: None,
        token_usage: AiTokenUsage::default(),
        session_token_usage: AiTokenUsage::default(),
        session_tree: SessionTreeNavigator::new(),
        background_jobs: Vec::new(),
        onboarding: None,
        terminal_size: (0, 0),
        clear_input_confirm: ClearInputConfirm::default(),
        model_picker: None,
        agent_start_time: None,
        input_history: Vec::new(),
        input_history_index: None,
        input_draft: String::new(),
        status_header: None,
        status_details: None,
        status_start_time: None,
        thinking_start: None,
        thinking_duration: None,
        is_thinking: false,
        mock_mode: false,
    }
}

pub fn make_state_with_text(text: &str) -> AppState {
    let state = AppState {
        messages: vec![],
        textarea: TextArea::new(vec![text.to_string()]),
        input_right_info: String::new(),
        mode: TuiMode::Chat,
        running: true,
        show_sidebar: false,
        agent_running: false,
        current_model: Some("gpt-4".to_string()),
        top_bar: TopBarState::default(),
        permission_modal: PermissionModalState::default(),
        command_palette: CommandPaletteState::default(),
        scroll: ScrollState::default(),
        animation: AnimationState::default(),
        diff_viewer: None,
        token_usage: AiTokenUsage::default(),
        session_token_usage: AiTokenUsage::default(),
        session_tree: SessionTreeNavigator::new(),
        background_jobs: Vec::new(),
        onboarding: None,
        terminal_size: (0, 0),
        clear_input_confirm: ClearInputConfirm::default(),
        model_picker: None,
        agent_start_time: None,
        input_history: Vec::new(),
        input_history_index: None,
        input_draft: String::new(),
        status_header: None,
        status_details: None,
        status_start_time: None,
        thinking_start: None,
        thinking_duration: None,
        is_thinking: false,
        mock_mode: false,
    };
    state
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
