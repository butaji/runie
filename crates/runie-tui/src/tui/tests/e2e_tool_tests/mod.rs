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

/// Creates a test AppState and CommandPalette together.
pub fn create_test_app() -> (AppState, CommandPalette) {
    (make_state(), CommandPalette::new())
}

/// Sends a permission request event and updates state/palette.
pub fn send_permission_request(
    state: &mut AppState,
    palette: &mut CommandPalette,
    tool_call_id: &str,
    tool_name: &str,
    tool_args: &str,
    tool_description: &str,
) {
    update(
        state,
        palette,
        Msg::AgentEvent(AgentEvent::PermissionRequest {
            tool_call_id: tool_call_id.to_string(),
            tool_name: tool_name.to_string(),
            tool_args: tool_args.to_string(),
            tool_description: tool_description.to_string(),
            turn: 1,
            context_window_usage: 0.0,
        }),
    );
}

/// Sends a tool execution start event.
pub fn send_tool_execution_start(
    state: &mut AppState,
    palette: &mut CommandPalette,
    tool_call_id: &str,
    tool_name: &str,
    tool_args: &str,
) {
    update(
        state,
        palette,
        Msg::AgentEvent(AgentEvent::ToolExecutionStart {
            tool_call_id: tool_call_id.to_string(),
            tool_name: tool_name.to_string(),
            tool_args: tool_args.to_string(),
            turn: 1,
        }),
    );
}

/// Sends a tool execution end event.
pub fn send_tool_execution_end(
    state: &mut AppState,
    palette: &mut CommandPalette,
    tool_call_id: &str,
    tool_name: &str,
    tool_args: &str,
    output_text: &str,
    is_error: bool,
    duration_ms: u64,
) {
    let tool_result = runie_agent::events::ToolResult {
        tool_call_id: tool_call_id.to_string(),
        tool_name: tool_name.to_string(),
        input: serde_json::json!({}),
        content: vec![ContentPart::Text {
            text: output_text.to_string(),
        }],
        is_error,
    };
    update(
        state,
        palette,
        Msg::AgentEvent(AgentEvent::ToolExecutionEnd {
            tool_call_id: tool_call_id.to_string(),
            tool_name: tool_name.to_string(),
            tool_args: tool_args.to_string(),
            result: tool_result,
            duration_ms,
            turn: 1,
        }),
    );
}

/// Verifies the last message contains the expected text.
pub fn verify_last_tool_result_contains(state: &AppState, expected: &str) -> bool {
    state.messages.iter().any(|m| {
        if let MessageItem::ToolCall {
            result: Some(r), ..
        } = m
        {
            r.contains(expected)
        } else {
            false
        }
    })
}

mod tool_lifecycle;
mod tool_permission;
mod tool_error;
mod tool_display;

pub use tool_lifecycle::*;
pub use tool_permission::*;
pub use tool_error::*;
pub use tool_display::*;
