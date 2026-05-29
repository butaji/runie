//! Agent domain update functions.
//! Handles: agent events, permissions.

pub mod events;
pub mod permission;
pub mod error;

use crate::components::MessageItem;
use crate::tui::state::{AppState, Msg, Cmd, TuiMode, PendingPermission};
use runie_agent::{AgentEvent, AgentMessage, ContentPart, PermissionDecision};
use runie_ai::TokenUsage;

pub use events::{update, handle_agent_event, on_message_start, on_message, on_message_update,
    on_message_end, update_last_assistant, on_tool_start, on_tool_end, on_agent_end,
    extract_text_content};

/// Agent-specific commands returned by update functions.
#[derive(Debug, Clone)]
pub enum AgentCmd {
    SendPermission { decision: PermissionDecision },
    Rollback { tool_call_id: String },
    FetchModels { provider_id: String, api_key: String },
}

impl From<AgentCmd> for Cmd {
    fn from(cmd: AgentCmd) -> Self {
        match cmd {
            AgentCmd::SendPermission { decision } => Cmd::SendPermission { decision },
            AgentCmd::Rollback { tool_call_id } => Cmd::Rollback { tool_call_id },
            AgentCmd::FetchModels { provider_id, api_key } => Cmd::FetchModels { provider_id, api_key },
        }
    }
}

pub fn to_agent_messages(items: &[MessageItem]) -> Vec<AgentMessage> {
    items.iter().filter_map(|item| match item {
        MessageItem::User { text, .. } => Some(AgentMessage {
            role: "user".to_string(),
            content: vec![ContentPart::Text { text: text.clone() }],
            timestamp: 0,
            usage: None,
            stop_reason: None,
            error_message: None,
            tool_calls: vec![],
        }),
        MessageItem::Assistant { text, .. } => Some(AgentMessage {
            role: "assistant".to_string(),
            content: vec![ContentPart::Text { text: text.clone() }],
            timestamp: 0,
            usage: None,
            stop_reason: None,
            error_message: None,
            tool_calls: vec![],
        }),
        // P2-1: Don't include Error messages in agent context
        MessageItem::Error { .. } => None,
        _ => None,
    }).collect()
}

/// P2-5 FIX: Handle permission request timeout - send denial to agent
pub fn handle_permission_timeout(state: &mut AppState) -> Vec<Cmd> {
    state.permission_modal.timed_out = true;
    // P2-5 FIX: Show timeout message before denial
    state.messages.push(MessageItem::System {
        text: "Permission request timed out after 5 minutes.".to_string(),
    });
    
    let tool_call_id = state.permission_modal.tool_call_id.clone().unwrap_or_default();
    state.permission_modal.tool = None;
    state.permission_modal.tool_call_id = None;

    // BG-1 FIX: Process next pending permission if any
    if !state.permission_modal.pending_queue.is_empty() {
        let pending = state.permission_modal.pending_queue.remove(0);
        state.permission_modal.tool = Some(pending.tool_name.clone());
        state.permission_modal.tool_call_id = Some(pending.tool_call_id.clone());
        state.permission_modal.args = Some(pending.tool_args.clone());
        state.permission_modal.desc = Some(format!("Agent wants to execute '{}'", pending.tool_name));
        state.permission_modal.timeout_start = Some(std::time::Instant::now());
        state.permission_modal.timed_out = false;
        state.mode = TuiMode::Permission;
    } else {
        state.mode = TuiMode::Chat;
    }

    let tool_name = state.permission_modal.tool.clone().unwrap_or_default();
    let tool_args = state.permission_modal.args.clone().unwrap_or_default();
    vec![Cmd::SendPermission { decision: PermissionDecision::Deny { tool_call_id, tool_name, tool_args } }]
}
