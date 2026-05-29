//! End-to-end flow tests for complete user interaction scenarios.
//!
//! These tests verify the full integration of state management, message handling,
//! and command generation across all domains (chat, agent, UI, onboarding).

#![allow(clippy::unwrap_used)]
#![cfg(test)]

use crate::tui::state::{AppState, Msg, Cmd, TuiMode, TopBarState, OnboardingStep};
use crate::components::{CommandPalette, MessageItem};
use crate::tui::update::update;
use runie_agent::{AgentEvent, AgentMessage, ContentPart, ToolResult, PermissionDecision};
use serde_json::json;

// ─── Test Helpers ───────────────────────────────────────────────────────────────

pub fn make_state() -> AppState {
    AppState::default()
}

pub fn make_state_with_model(model: &str) -> AppState {
    AppState {
        current_model: Some(model.to_string()),
        top_bar: TopBarState {
            model: model.to_string(),
            ..Default::default()
        },
        ..Default::default()
    }
}

pub fn make_state_with_text(text: &str) -> AppState {
    AppState {
        current_model: Some("openai/gpt-4o".to_string()),
        textarea: ratatui_textarea::TextArea::new(vec![text.to_string()]),
        ..Default::default()
    }
}

pub fn make_agent_message(role: &str, content: &str) -> AgentMessage {
    AgentMessage {
        role: role.to_string(),
        content: vec![ContentPart::Text { text: content.to_string() }],
        timestamp: 0,
        usage: None,
        stop_reason: None,
        error_message: None,
        tool_calls: vec![],
    }
}

pub fn make_tool_result(tool_call_id: &str, tool_name: &str, content: &str, is_error: bool) -> ToolResult {
    ToolResult {
        tool_call_id: tool_call_id.to_string(),
        tool_name: tool_name.to_string(),
        input: json!({}),
        content: vec![ContentPart::Text { text: content.to_string() }],
        is_error,
    }
}

mod onboarding_flows;
mod chat_flows;
mod agent_flows;
mod tool_flows;
mod permission_flows;
mod palette_flows;
mod settings_flows;
mod mode_transitions;
mod ui_flows;
mod slash_commands;
mod cursor_and_animation;
mod git_info;

pub use onboarding_flows::*;
pub use chat_flows::*;
pub use agent_flows::*;
pub use tool_flows::*;
pub use permission_flows::*;
pub use palette_flows::*;
pub use settings_flows::*;
pub use mode_transitions::*;
pub use ui_flows::*;
pub use slash_commands::*;
pub use cursor_and_animation::*;
pub use git_info::*;
