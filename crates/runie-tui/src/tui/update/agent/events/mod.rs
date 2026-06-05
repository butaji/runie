//! Agent event handlers.

mod thinking;
pub mod events_message;
pub mod events_turn;

use crate::components::MessageItem;
use crate::messages::MessageRegistry;
use crate::tui::state::{AppState, TuiMode};
use runie_agent::{AgentEvent, ContentPart};
use runie_ai::TokenUsage;

pub use events_message::{on_message, on_message_end, on_message_start, on_message_update};
pub use events_turn::{on_turn_end, update_last_assistant};
use thinking::{ensure_thinking_placeholder, on_thinking_end, on_thinking_start, on_thinking_update};

fn current_timestamp() -> Option<String> {
    use chrono::Local;
    if let Ok(mock) = std::env::var("RUNIE_MOCK_TIMESTAMP") {
        if !mock.is_empty() { return Some(mock); }
    }
    Some(Local::now().format("%-I:%M %p").to_string())
}

/// Update agent domain: agent events, permissions.
pub fn update(state: &mut AppState, msg: crate::tui::state::Msg) -> Vec<crate::tui::update::agent::AgentCmd> {
    match msg {
        crate::tui::state::Msg::AgentEvent(event) => {
            handle_agent_event(state, event)
        }
        crate::tui::state::Msg::PermissionConfirm | crate::tui::state::Msg::PermissionCancel |
        crate::tui::state::Msg::PermissionAlways | crate::tui::state::Msg::PermissionSkip => {
            return super::permission::handle_permission_msg(state, msg);
        }
        _ => vec![],
    }
}

pub fn handle_agent_event(state: &mut AppState, event: AgentEvent) -> Vec<super::AgentCmd> {
    match categorize_event(&event) {
        EventCategory::Message(msg) => { handle_message_event(state, msg); vec![] }
        EventCategory::Tool(tool) => handle_tool_event(state, tool),
        EventCategory::Lifecycle(lifecycle) => { handle_lifecycle_event(state, lifecycle); vec![] }
        EventCategory::Error(err) => { super::super::agent::error::on_agent_error(state, err); vec![] }
        EventCategory::Token(tokens) => { update_token_usage(state, tokens.0, tokens.1); vec![] }
        EventCategory::Permission(perm) => handle_permission_event(state, perm),
        EventCategory::Thinking(thinking) => { handle_thinking_event(state, thinking); vec![] }
        EventCategory::Ignored => vec![]
    }
}

/// Event categories for routing
enum EventCategory {
    Message(AgentEvent),
    Tool(AgentEvent),
    Lifecycle(AgentEvent),
    Error(String),
    Token(TokenEvent),
    Permission(AgentEvent),
    Thinking(AgentEvent),
    Ignored,
}

/// Token usage event data
struct TokenEvent(usize, usize);

fn categorize_event(event: &AgentEvent) -> EventCategory {
    match event {
        AgentEvent::Message { .. } | AgentEvent::MessageStart { .. } |
        AgentEvent::MessageUpdate { .. } | AgentEvent::MessageEnd { .. } => EventCategory::Message(event.clone()),
        AgentEvent::ToolExecutionStart { .. } | AgentEvent::ToolExecutionEnd { .. } => EventCategory::Tool(event.clone()),
        AgentEvent::AgentEnd { .. } | AgentEvent::TurnEnd { .. } => EventCategory::Lifecycle(event.clone()),
        AgentEvent::Error { message, .. } => EventCategory::Error(message.clone()),
        AgentEvent::TokenUsage { prompt_tokens, completion_tokens, .. } =>
            EventCategory::Token(TokenEvent(*prompt_tokens, *completion_tokens)),
        AgentEvent::PermissionRequest { .. } => EventCategory::Permission(event.clone()),
        AgentEvent::ThinkingStart { .. } | AgentEvent::ThinkingUpdate { .. } |
        AgentEvent::ThinkingEnd { .. } => EventCategory::Thinking(event.clone()),
        AgentEvent::ContextCompacted { .. } => EventCategory::Ignored,
    }
}

// ─── Category handlers ──────────────────────────────────────────────────────

fn handle_message_event(state: &mut AppState, event: AgentEvent) {
    match event {
        AgentEvent::Message { role, content } => on_message(state, &role, &content),
        AgentEvent::MessageStart { message, .. } => on_message_start(state, message),
        AgentEvent::MessageUpdate { message, delta, replace, .. } => on_message_update(state, message, &delta, replace),
        AgentEvent::MessageEnd { message, .. } => on_message_end(state, message),
        _ => {}
    }
}

fn handle_thinking_event(state: &mut AppState, event: AgentEvent) {
    match event {
        AgentEvent::ThinkingStart { turn } => on_thinking_start(state, turn),
        AgentEvent::ThinkingUpdate { delta, .. } => on_thinking_update(state, &delta),
        AgentEvent::ThinkingEnd { duration_ms, .. } => on_thinking_end(state, duration_ms),
        _ => {}
    }
}

fn handle_tool_event(state: &mut AppState, event: AgentEvent) -> Vec<super::AgentCmd> {
    match event {
        AgentEvent::ToolExecutionStart { tool_call_id, tool_name, tool_args, .. } => {
            handle_tool_start(state, tool_call_id, tool_name, serde_json::Value::String(tool_args))
        }
        AgentEvent::ToolExecutionEnd { tool_name, result, .. } => {
            handle_tool_end(state, tool_name, result)
        }
        _ => vec![],
    }
}

fn handle_tool_start(
    state: &mut AppState,
    tool_call_id: String,
    tool_name: String,
    tool_args: serde_json::Value,
) -> Vec<super::AgentCmd> {
    let tool_args_str = match &tool_args {
        serde_json::Value::String(s) => s.clone(),
        _ => tool_args.to_string(),
    };
    on_tool_start(state, tool_call_id, tool_name.clone(), tool_args_str);
    let ext_event = runie_ext::PluginEvent::ToolCalled {
        tool_name,
        arguments: serde_json::json!({"args": tool_args }),
    };
    log_plugin_actions(state, ext_event);
    vec![]
}

fn handle_tool_end(
    state: &mut AppState,
    tool_name: String,
    result: runie_agent::events::ToolResult,
) -> Vec<super::AgentCmd> {
    let is_error = result.is_error;
    let content = extract_text_content(&result.content);
    on_tool_end_with_text(state, result, content.clone());
    let ext_event = runie_ext::PluginEvent::ToolResult {
        tool_name,
        output: runie_core::ToolOutput {
            content,
            metadata: serde_json::json!({"is_error": is_error}),
            terminate: false,
        },
    };
    log_plugin_actions(state, ext_event);
    vec![]
}

fn log_plugin_actions(state: &AppState, event: runie_ext::PluginEvent) {
    let actions = state.extension_registry.dispatch_event(event);
    for action in &actions {
        tracing::debug!("Plugin action: {:?}", action);
    }
}

fn handle_lifecycle_event(state: &mut AppState, event: AgentEvent) {
    match event {
        AgentEvent::AgentEnd { .. } => on_agent_end(state),
        AgentEvent::TurnEnd { turn_duration_ms, .. } => on_turn_end(state, turn_duration_ms),
        _ => {}
    }
}

fn handle_permission_event(state: &mut AppState, event: AgentEvent) -> Vec<super::AgentCmd> {
    if let AgentEvent::PermissionRequest { tool_call_id, tool_name, tool_args, .. } = event {
        super::permission::on_permission_request(state, tool_call_id, tool_name, tool_args)
    } else {
        vec![]
    }
}

fn update_token_usage(state: &mut AppState, prompt_tokens: usize, completion_tokens: usize) {
    state.session_token_usage.prompt_tokens += prompt_tokens;
    state.session_token_usage.completion_tokens += completion_tokens;
    state.session_token_usage.total_tokens += prompt_tokens + completion_tokens;
    if let Some(ref model) = state.current_model {
        let cost = TokenUsage::estimate_cost(prompt_tokens, completion_tokens, model);
        state.session_token_usage.estimated_cost += cost;
    }
}

// ─── Tool handlers ───────────────────────────────────────────────────────────

pub fn on_tool_start(state: &mut AppState, _tool_call_id: String, tool_name: String, tool_args: String) {
    state.agent_running = true;
    state.session_starting = None;
    if let Some(ref mut thinking) = state.thinking {
        if let Some(start) = thinking.start.take() {
            let elapsed = start.elapsed();
            thinking.accrued_duration = Some(thinking.accrued_duration.unwrap_or_default() + elapsed);
            thinking.start = Some(std::time::Instant::now());
        }
    }
    state.status_header = Some(MessageRegistry::status_running().to_string());
    let elapsed = state.status_start_time.map(|t| t.elapsed().as_secs()).unwrap_or(0);
    state.status_details = Some(MessageRegistry::format_elapsed(elapsed));
    state.messages.push(MessageItem::ToolRunning {
        name: tool_name,
        args: tool_args,
        duration_ms: 0,
        total_elapsed_ms: 0,
        download_bytes: 0,
    });
}

pub fn on_tool_end_with_text(
    state: &mut AppState,
    tool_result: runie_agent::events::ToolResult,
    precomputed_text: String,
) {
    apply_tool_result(state, &tool_result.tool_name, precomputed_text, tool_result.is_error);
}

pub fn on_tool_end(state: &mut AppState, tool_result: runie_agent::events::ToolResult) {
    let text = extract_text_content(&tool_result.content);
    apply_tool_result(state, &tool_result.tool_name, text, tool_result.is_error);
}

fn apply_tool_result(state: &mut AppState, tool_name: &str, text: String, err: bool) {
    let pos = state.messages.iter().rposition(|m| match m {
        MessageItem::ToolCall { name, .. } | MessageItem::ToolRunning { name, .. } => name == tool_name,
        _ => false,
    });
    if let Some(idx) = pos {
        if let Some(msg) = state.messages.get_mut(idx) {
            match msg {
                MessageItem::ToolCall { result, is_error, .. } => {
                    *result = Some(text);
                    *is_error = err;
                }
                MessageItem::ToolRunning { name, args, .. } => {
                    let name = std::mem::take(name);
                    let args = std::mem::take(args);
                    *msg = MessageItem::ToolCall { name, args, result: Some(text), is_error: err };
                }
                _ => {}
            }
        }
    }
}

// ─── Lifecycle handlers ─────────────────────────────────────────────────────

pub fn on_agent_end(state: &mut AppState) {
    state.agent_running = false;
    state.thinking = None;
    state.agent_start_time = None;
    state.status_header = None;
    state.status_details = None;
    state.status_start_time = None;
    state.session_starting = None;
    if state.mode == TuiMode::Permission {
        state.permission_modal.tool = None;
        state.permission_modal.tool_call_id = None;
    }
    state.permission_modal.pending_queue.clear();
    state.mode = TuiMode::Chat;
    if let Some(MessageItem::Assistant { text, .. }) = state.messages.last() {
        if text.is_empty() {
            state.messages.pop();
            state.messages.push(MessageItem::System {
                text: "Agent completed but produced no response.".to_string(),
            });
        }
    }
}

// ─── Utility ─────────────────────────────────────────────────────────────────

pub fn extract_text_content(parts: &[ContentPart]) -> String {
    parts.iter()
        .filter_map(|part| {
            if let ContentPart::Text { text } = part { Some(text.as_str()) } else { None }
        })
        .collect::<Vec<_>>()
        .join("")
}
