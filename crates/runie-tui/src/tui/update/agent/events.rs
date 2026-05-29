//! Agent event handlers.

use crate::components::MessageItem;
use crate::tui::state::{AppState, TuiMode};
use runie_agent::{AgentEvent, ContentPart};
use runie_ai::TokenUsage;

/// Update agent domain: agent events, permissions.
pub fn update(state: &mut AppState, msg: crate::tui::state::Msg) -> Vec<crate::AgentCmd> {
    match msg {
        crate::tui::state::Msg::AgentEvent(event) => {
            handle_agent_event(state, event);
            vec![]
        }
        crate::tui::state::Msg::PermissionConfirm | crate::tui::state::Msg::PermissionCancel |
        crate::tui::state::Msg::PermissionAlways | crate::tui::state::Msg::PermissionSkip => {
            return super::permission::handle_permission_msg(state, msg);
        }
        _ => vec![],
    }
}

pub fn handle_agent_event(state: &mut AppState, event: AgentEvent) {
    // Route to category handlers to reduce match complexity
    if let Some(msg_event) = extract_message_event(&event) {
        handle_message_event(state, msg_event);
    } else if let Some(tool_event) = extract_tool_event(&event) {
        handle_tool_event(state, tool_event);
    } else if let Some(lifecycle_event) = extract_lifecycle_event(&event) {
        handle_lifecycle_event(state, lifecycle_event);
    } else if let Some(error_event) = extract_error_event(&event) {
        super::error::on_agent_error(state, error_event.0);
    } else if let Some(token_event) = extract_token_event(&event) {
        update_token_usage(state, token_event.0, token_event.1);
    }
    // No-op events: PermissionGranted, PermissionDenied, ContextCompacted - ignored
}

// ─── Event extractors (reduce match complexity by categorizing) ─────────────

struct ErrorEvent(String);
struct TokenEvent(usize, usize);

fn extract_message_event(event: &AgentEvent) -> Option<AgentEvent> {
    match event {
        AgentEvent::Message { .. } | AgentEvent::MessageStart { .. } |
        AgentEvent::MessageUpdate { .. } | AgentEvent::MessageEnd { .. } => Some(event.clone()),
        _ => None,
    }
}

fn extract_tool_event(event: &AgentEvent) -> Option<AgentEvent> {
    match event {
        AgentEvent::ToolExecutionStart { .. } | AgentEvent::ToolExecutionEnd { .. } => Some(event.clone()),
        _ => None,
    }
}

fn extract_lifecycle_event(event: &AgentEvent) -> Option<AgentEvent> {
    match event {
        AgentEvent::AgentEnd { .. } | AgentEvent::TurnEnd { .. } => Some(event.clone()),
        _ => None,
    }
}

fn extract_error_event(event: &AgentEvent) -> Option<ErrorEvent> {
    match event {
        AgentEvent::Error { message, .. } => Some(ErrorEvent(message.clone())),
        _ => None,
    }
}

fn extract_token_event(event: &AgentEvent) -> Option<TokenEvent> {
    match event {
        AgentEvent::TokenUsage { prompt_tokens, completion_tokens, .. } =>
            Some(TokenEvent(*prompt_tokens, *completion_tokens)),
        _ => None,
    }
}

// ─── Category handlers ──────────────────────────────────────────────────────

fn handle_message_event(state: &mut AppState, event: AgentEvent) {
    match event {
        AgentEvent::Message { role, content } => on_message(state, &role, &content),
        AgentEvent::MessageStart { message, .. } => on_message_start(state, message),
        AgentEvent::MessageUpdate { message, .. } => on_message_update(state, message),
        AgentEvent::MessageEnd { message, .. } => on_message_end(state, message),
        _ => {}
    }
}

fn handle_tool_event(state: &mut AppState, event: AgentEvent) {
    match event {
        AgentEvent::ToolExecutionStart { tool_call_id, .. } => on_tool_start(state, tool_call_id),
        AgentEvent::ToolExecutionEnd { result, .. } => on_tool_end(state, result),
        _ => {}
    }
}

fn handle_lifecycle_event(state: &mut AppState, event: AgentEvent) {
    match event {
        AgentEvent::AgentEnd { .. } => on_agent_end(state),
        AgentEvent::TurnEnd { .. } => on_turn_end(state),
        _ => {}
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

// ─── Message handlers ───────────────────────────────────────────────────────

pub fn on_message_start(state: &mut AppState, _message: runie_agent::events::AgentMessage) {
    state.agent_running = true;
    state.status_header = Some("Thinking".to_string());
    state.status_details = None;
    state.status_start_time = Some(std::time::Instant::now());
    // Track thinking duration
    state.is_thinking = true;
    state.thinking_start = Some(std::time::Instant::now());
    state.thinking_duration = None;
    // Auto-scroll to bottom if user hasn't scrolled up
    if !state.scroll.user_scrolled_up {
        state.scroll.feed_offset = 0;
    }
    // NOTE: Do NOT overwrite current_model here - it contains the user's configured model
    // and must persist across agent runs. The model used per message is tracked separately.
    // Skip pushing new assistant if placeholder already exists (added in handle_submit)
    let has_placeholder = state.messages.last()
        .map(|m| matches!(m, MessageItem::Assistant { text, .. } if text.is_empty()))
        .unwrap_or(false);
    if !has_placeholder {
        state.messages.push(MessageItem::Assistant {
            text: String::new(),
            model: state.current_model.clone(),
            timestamp: None,
        });
    }
}

pub fn on_message(state: &mut AppState, role: &str, content: &str) {
    match role {
        "user" => state.messages.push(MessageItem::User {
            text: content.to_string(),
            model: Some("You".to_string()),
            timestamp: None,
        }),
        "assistant" => state.messages.push(MessageItem::Assistant {
            text: content.to_string(),
            model: state.current_model.clone(),
            timestamp: None,
        }),
        "system" => state.messages.push(MessageItem::System { text: content.to_string() }),
        _ => state.messages.push(MessageItem::System { text: content.to_string() }),
    }
}

pub fn on_message_update(state: &mut AppState, message: runie_agent::events::AgentMessage) {
    // Auto-scroll to bottom if user hasn't scrolled up
    if !state.scroll.user_scrolled_up {
        state.scroll.feed_offset = 0;
    }
    update_last_assistant(state, &message.content);
}

pub fn on_message_end(state: &mut AppState, message: runie_agent::events::AgentMessage) {
    // Calculate and record thinking duration
    if let Some(start) = state.thinking_start.take() {
        state.thinking_duration = Some(start.elapsed());
        state.is_thinking = false;
    }
    // Add thinking indicator if thinking took more than 0.5s
    if let Some(duration) = state.thinking_duration {
        let secs = duration.as_secs_f32();
        if secs > 0.5 {
            state.messages.push(MessageItem::Thought { duration_secs: secs });
        }
    }
    // Auto-scroll to bottom if user hasn't scrolled up
    if !state.scroll.user_scrolled_up {
        state.scroll.feed_offset = 0;
    }
    update_last_assistant(state, &message.content);
}

/// Handle turn end - add separator with runtime metrics
fn on_turn_end(state: &mut AppState) {
    // Add separator if we have timing info
    if let Some(start_time) = state.agent_start_time {
        let elapsed = start_time.elapsed().as_secs();
        let tool_calls = state.messages.iter().filter(|m| {
            matches!(m, MessageItem::ToolCall { .. })
        }).count();

        state.messages.push(MessageItem::Separator {
            elapsed_secs: elapsed,
            tool_calls,
            tokens_used: Some(state.session_token_usage.total_tokens),
        });
    }
}

pub fn update_last_assistant(state: &mut AppState, content: &[ContentPart]) {
    if let Some(MessageItem::Assistant { ref mut text, .. }) = state.messages.last_mut() {
        *text = extract_text_content(content);
    }
}

// ─── Tool handlers ───────────────────────────────────────────────────────────

pub fn on_tool_start(state: &mut AppState, tool_call_id: String) {
    // Pause thinking timer when tool starts - accumulate duration so far
    if state.is_thinking {
        if let Some(start) = state.thinking_start.take() {
            let elapsed = start.elapsed();
            state.thinking_duration = Some(elapsed);
            state.is_thinking = false;
        }
    }
    state.status_header = Some("Working".to_string());
    state.status_details = Some(format!("Running {}", tool_call_id));
    state.messages.push(MessageItem::ToolCall {
        name: tool_call_id,
        args: String::new(),
        result: None,
        is_error: false,
    });
}

pub fn on_tool_end(state: &mut AppState, tool_result: runie_agent::events::ToolResult) {
    let text = extract_text_content(&tool_result.content);
    if let Some(MessageItem::ToolCall { ref mut result, ref mut is_error, .. }) = state.messages.last_mut() {
        *result = Some(text);
        *is_error = tool_result.is_error;
    }
}

// ─── Lifecycle handlers ─────────────────────────────────────────────────────

pub fn on_agent_end(state: &mut AppState) {
    state.agent_running = false;
    // P0-AGENT-TIMEOUT: Clear agent start time on end
    state.agent_start_time = None;
    // Clear live status
    state.status_header = None;
    state.status_details = None;
    state.status_start_time = None;
    // NOTE: Do not clear current_model - it contains the user's configured model
    // and must persist across agent runs for subsequent submissions.
    // BG-5 FIX: Clear any pending permission modal
    if state.mode == TuiMode::Permission {
        state.permission_modal.tool = None;
        state.permission_modal.tool_call_id = None;
    }
    // BG-1 FIX: Clear pending permission queue when agent ends
    state.permission_modal.pending_queue.clear();
    state.mode = TuiMode::Chat;
}

// ─── Utility ─────────────────────────────────────────────────────────────────

pub fn extract_text_content(parts: &[ContentPart]) -> String {
    parts.iter()
        .filter_map(|part| {
            if let ContentPart::Text { text } = part {
                Some(text.as_str())
            } else {
                None
            }
        })
        .collect::<Vec<_>>()
        .join("")
}
