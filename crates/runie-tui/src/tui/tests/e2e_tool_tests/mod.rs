//! End-to-end tests for tool execution lifecycle.
//!
//! These tests verify the full tool call flow from agent request through
//! TUI display, including permission handling, execution, and result rendering.

use crate::tui::state::{AppState, AnimationState, CommandPaletteState, Msg, Cmd, ScrollState, TopBarState, PermissionModalState, TuiMode, ClearInputConfirm};
use crate::components::{MessageItem, CommandPalette};
use crate::tui::update::update;
use runie_agent::{AgentEvent, PermissionDecision, ContentPart};
use runie_ai::TokenUsage as AiTokenUsage;

use ratatui_textarea::TextArea;

pub fn make_state() -> AppState {
    AppState {
        messages: vec![],
        textarea: TextArea::default(),
        input_right_info: String::new(),
        mode: TuiMode::Chat,
        running: true,
        show_sidebar: false,
        agent_running: false,
        current_model: Some("test-model".to_string()),
        top_bar: TopBarState::default(),
        permission_modal: PermissionModalState::default(),
        command_palette: CommandPaletteState::default(),
        scroll: ScrollState::default(),
        animation: AnimationState::default(),
        diff_viewer: None,
        token_usage: AiTokenUsage::default(),
        session_token_usage: AiTokenUsage::default(),
        session_tree: Default::default(),
        background_jobs: Vec::new(),
        onboarding: None,
        terminal_size: (80, 24),
        clear_input_confirm: ClearInputConfirm::default(),
        model_picker: None,
        agent_start_time: None,
    }
}

mod tool_lifecycle;
mod tool_permission;
mod tool_error;
mod tool_display;

pub use tool_lifecycle::*;
pub use tool_permission::*;
pub use tool_error::*;
pub use tool_display::*;
