//! Agent event handlers.

mod thinking;

use crate::components::MessageItem;
use crate::messages::MessageRegistry;
use crate::tui::state::{AppState, TuiMode, ThinkingState};
use runie_agent::{AgentEvent, ContentPart};
use runie_ai::TokenUsage;

use thinking::{ensure_thinking_placeholder, on_thinking_end, on_thinking_start, on_thinking_update};

fn current_timestamp() -> Option<String> {
    use chrono::Local;
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
    // Route to category handlers - uses match to keep complexity low
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

/// Categorize an agent event into a routing category
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
        // Ignored events
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
    let content = extract_text_from_content(&result.content);
    on_tool_end(state, result);
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

fn extract_text_from_content(parts: &[runie_agent::events::ContentPart]) -> String {
    // Delegates to the public extractor (same logic).
    extract_text_content(parts)
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
        AgentEvent::TurnEnd { .. } => on_turn_end(state),
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

// ─── Message handlers ───────────────────────────────────────────────────────

pub fn on_message_start(state: &mut AppState, _message: runie_agent::events::AgentMessage) {
    tracing::debug!("on_message_start: setting agent_running = true");
    state.agent_running = true;
    state.session_starting = None; // Clear "Starting session..." indicator
    state.status_header = Some(MessageRegistry::status_thinking().to_string());
    state.status_start_time = Some(std::time::Instant::now());
    // Set status_details to show elapsed time from start
    state.status_details = Some(MessageRegistry::format_elapsed(0));
    // Track thinking duration
    state.thinking = Some(ThinkingState {
        start: Some(std::time::Instant::now()),
        text: String::new(),
        accrued_duration: None,
    });
    // Clear previous turn info - new turn starting
    state.last_turn_duration_secs = None;
    state.last_turn_tokens = None;
    state.last_turn_tool_calls = None;
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
            timestamp: current_timestamp(),
            expanded: true,
            thought_duration: None,
            turn_duration: None,
        });
    }
}

pub fn on_message(state: &mut AppState, role: &str, content: &str) {
    match role {
        "user" => state.messages.push(MessageItem::User {
            text: content.to_string(),
            model: Some("You".to_string()),
            timestamp: current_timestamp(),
        }),
        "assistant" => state.messages.push(MessageItem::Assistant {
            text: content.to_string(),
            model: state.current_model.clone(),
            timestamp: current_timestamp(),
            expanded: true,
            thought_duration: None,
            turn_duration: None,
        }),
        "system" => {
            // Filter out system messages that are just metadata notifications
            if !content.starts_with("Using ") && !content.starts_with("Mock mode") {
                state.messages.push(MessageItem::System { text: content.to_string() });
            }
        }
        _ => state.messages.push(MessageItem::System { text: content.to_string() }),
    }
}

pub fn on_message_update(state: &mut AppState, message: runie_agent::events::AgentMessage, delta: &str, replace: bool) {
    // Auto-scroll to bottom if user hasn't scrolled up
    if !state.scroll.user_scrolled_up {
        state.scroll.feed_offset = 0;
    }

    ensure_thinking_placeholder(state, &message.content);
    if replace {
        // Test snapshot mode: replace text wholesale.
        if let Some(MessageItem::Assistant { ref mut text, .. }) = state.messages.last_mut() {
            *text = delta.to_string();
        }
    } else if !delta.is_empty() {
        // Live streaming: append only the new characters.
        if let Some(MessageItem::Assistant { ref mut text, .. }) = state.messages.last_mut() {
            text.push_str(delta);
        }
    }
}

pub fn on_message_end(state: &mut AppState, message: runie_agent::events::AgentMessage) {
    // Clear agent_running -- the assistant has finished streaming this
    // message. (on_agent_end clears it again at turn end, but setting
    // it here too means the top bar spinner and global_tags "Thinking"
    // indicator stop showing once the assistant finishes, which
    // matches Grok's behavior in 05_input_typing.)
    state.agent_running = false;
    // Calculate and record thinking duration (text is dropped --
    // state.thinking = None below reclaims the full struct)
    let duration = if let Some(ref mut thinking) = state.thinking {
        let current_duration = thinking.start.take().map(|start| start.elapsed());
        // Include any accrued duration from tools
        current_duration.map(|d| {
            thinking
                .accrued_duration
                .map(|acc| d + acc)
                .unwrap_or(d)
        })
    } else {
        None
    };
    state.thinking = None;

    // Auto-scroll to bottom if user hasn't scrolled up
    if !state.scroll.user_scrolled_up {
        state.scroll.feed_offset = 0;
    }

    // Update the assistant's text and set thought_duration if thinking took > 0.5s
    if let Some(duration) = duration {
        let secs = duration.as_secs_f32();
        if secs > 0.5 {
            // Update last assistant message with thought duration
            if let Some(MessageItem::Assistant { thought_duration: ref mut td, .. }) = state.messages.last_mut() {
                *td = Some(secs);
            }
        }
    }

    // Update assistant's text content
    update_last_assistant(state, &message.content);
}

/// Handle turn end - add separator with runtime metrics
fn on_turn_end(state: &mut AppState) {
    if let Some(start_time) = state.agent_start_time {
        let elapsed = start_time.elapsed().as_secs();
        let tool_calls = state.messages.iter().filter(|m| {
            matches!(m, MessageItem::ToolCall { .. })
        }).count();

        state.last_turn_duration_secs = Some(elapsed);
        state.last_turn_tokens = Some(state.session_token_usage.total_tokens);
        state.last_turn_tool_calls = Some(tool_calls);
        state.turn_success = Some(true);

        // Set turn_duration on the last assistant message
        if let Some(MessageItem::Assistant { turn_duration: ref mut td, .. }) = state.messages.last_mut() {
            *td = Some(elapsed as f32);
        }

        // Add separator to feed with grok-style metrics
        state.messages.push(MessageItem::Separator {
            elapsed_secs: elapsed,
            tool_calls,
            tokens_used: Some(state.session_token_usage.total_tokens),
        });
    }
}

pub fn update_last_assistant(state: &mut AppState, content: &[ContentPart]) {
    if let Some(MessageItem::Assistant { ref mut text, .. }) = state.messages.last_mut() {
        let new_content = extract_text_content(content);
        if !new_content.is_empty() {
            // Avoid duplicate appends - only add if not already suffix
            if !text.ends_with(&new_content) {
                // Only insert newline if text exists and doesn't end with one
                let needs_newline = !text.is_empty() && !text.ends_with('\n');
                if needs_newline {
                    text.push('\n');
                }
                text.push_str(&new_content);
            }
        }
    }
}

// ─── Tool handlers ───────────────────────────────────────────────────────────

pub fn on_tool_start(state: &mut AppState, _tool_call_id: String, tool_name: String, tool_args: String) {
    // Agent is actively working (executing a tool). Keep the top-bar
    // spinner and global_tags "Thinking..." indicator on. (Set here
    // because on_message_end clears it as soon as the text stream
    // ends, but a tool execution may follow immediately.)
    state.agent_running = true;
    // Clear "Starting session..." indicator - agent is now doing real work
    state.session_starting = None;
    // Pause thinking timer when tool starts - accumulate duration so far
    if let Some(ref mut thinking) = state.thinking {
        if let Some(start) = thinking.start.take() {
            let elapsed = start.elapsed();
            thinking.accrued_duration = Some(thinking.accrued_duration.unwrap_or_default() + elapsed);
            thinking.start = Some(std::time::Instant::now()); // Reset to track additional thinking time after tool
        }
    }
    state.status_header = Some(MessageRegistry::status_running().to_string());
    // Calculate elapsed time from status_start_time and set status_details
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

pub fn on_tool_end(state: &mut AppState, tool_result: runie_agent::events::ToolResult) {
    let text = extract_text_content(&tool_result.content);
    // Find the matching tool item by tool_call_id (preferred) or
    // tool_name (fallback). Don't blindly trust last_mut(): with
    // back-to-back tool calls, the wrong item could get the result.
    // on_tool_start creates ToolRunning; convert it to a completed
    // ToolCall so the renderer shows the result instead of the spinner.
    let pos = state
        .messages
        .iter()
        .position(|m| match m {
            MessageItem::ToolCall { .. } | MessageItem::ToolRunning { .. } => {
                // tool_call_id isn't stored on MessageItem; fall back
                // to tool_name match. Both the start and end events
                // carry the same tool_name in a single tool call.
                let item_name = match m {
                    MessageItem::ToolCall { name, .. } => name,
                    MessageItem::ToolRunning { name, .. } => name,
                    _ => unreachable!(),
                };
                item_name == &tool_result.tool_name
            }
            _ => false,
        });
    if let Some(idx) = pos {
        if let Some(msg) = state.messages.get_mut(idx) {
            match msg {
                MessageItem::ToolCall {
                    result, is_error, ..
                } => {
                    *result = Some(text);
                    *is_error = tool_result.is_error;
                }
                MessageItem::ToolRunning { name, args, .. } => {
                    let name = std::mem::take(name);
                    let args = std::mem::take(args);
                    *msg = MessageItem::ToolCall {
                        name,
                        args,
                        result: Some(text),
                        is_error: tool_result.is_error,
                    };
                }
                _ => {}
            }
        }
    }
}

// ─── Lifecycle handlers ─────────────────────────────────────────────────────

pub fn on_agent_end(state: &mut AppState) {
    state.agent_running = false;
    // Clear thinking state
    state.thinking = None;
    // P0-AGENT-TIMEOUT: Clear agent start time on end
    state.agent_start_time = None;
    // Clear live status
    state.status_header = None;
    state.status_details = None;
    state.status_start_time = None;
    // Clear session starting indicator - ensures subsequent turns are not blocked
    state.session_starting = None;
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
    // Remove empty assistant placeholder if agent finished with no content
    // and replace with a system notice so user knows something happened
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
            if let ContentPart::Text { text } = part {
                Some(text.as_str())
            } else {
                None
            }
        })
        .collect::<Vec<_>>()
        .join("")
}
