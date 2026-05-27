use crate::components::MessageItem;
use crate::tui::state::{AppState, Msg, Cmd, TuiMode, PendingPermission};
use runie_agent::{AgentEvent, AgentMessage, ContentPart, PermissionDecision};
use runie_ai::TokenUsage;

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

/// Update agent domain: agent events, permissions.
pub fn update(state: &mut AppState, msg: crate::tui::state::Msg) -> Vec<AgentCmd> {
    match msg {
        crate::tui::state::Msg::AgentEvent(event) => {
            handle_agent_event(state, event);
            vec![]
        }
        crate::tui::state::Msg::PermissionConfirm | crate::tui::state::Msg::PermissionCancel | crate::tui::state::Msg::PermissionAlways | crate::tui::state::Msg::PermissionSkip => {
            return handle_permission_msg(state, msg);
        }
        _ => vec![],
    }
}

pub fn handle_agent_event(state: &mut AppState, event: AgentEvent) {
    match event {
        AgentEvent::Message { role, content } => on_message(state, &role, &content),
        AgentEvent::MessageStart { message, .. } => on_message_start(state, message),
        AgentEvent::MessageUpdate { message, .. } => on_message_update(state, message),
        AgentEvent::MessageEnd { message, .. } => on_message_end(state, message),
        AgentEvent::ToolExecutionStart { tool_call_id, .. } => on_tool_start(state, tool_call_id),
        AgentEvent::ToolExecutionEnd { result, .. } => on_tool_end(state, result),
        AgentEvent::AgentEnd { .. } => on_agent_end(state),
        AgentEvent::Error { message, .. } => on_agent_error(state, message),
        AgentEvent::PermissionRequest { tool_call_id, tool_name, tool_args, .. } => on_permission_request(state, tool_call_id, tool_name, tool_args),
        AgentEvent::TokenUsage { prompt_tokens, completion_tokens, .. } => {
            state.session_token_usage.prompt_tokens += prompt_tokens;
            state.session_token_usage.completion_tokens += completion_tokens;
            state.session_token_usage.total_tokens += prompt_tokens + completion_tokens;
            if let Some(ref model) = state.current_model {
                let cost = TokenUsage::estimate_cost(prompt_tokens, completion_tokens, model);
                state.session_token_usage.estimated_cost += cost;
            }
        }
        AgentEvent::TurnEnd { .. } | AgentEvent::PermissionGranted { .. } | AgentEvent::PermissionDenied { .. } => {}
    }
}

pub fn on_message_start(state: &mut AppState, message: runie_agent::events::AgentMessage) {
    state.agent_running = true;
    state.current_model = Some(message.role.clone());
    state.messages.push(MessageItem::Assistant {
        text: String::new(),
        model: state.current_model.clone(),
        timestamp: None,
    });
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
    update_last_assistant(state, &message.content);
}

pub fn on_message_end(state: &mut AppState, message: runie_agent::events::AgentMessage) {
    update_last_assistant(state, &message.content);
}

pub fn update_last_assistant(state: &mut AppState, content: &[ContentPart]) {
    if let Some(MessageItem::Assistant { ref mut text, .. }) = state.messages.last_mut() {
        *text = extract_text_content(content);
    }
}

pub fn on_tool_start(state: &mut AppState, tool_call_id: String) {
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

pub fn on_agent_end(state: &mut AppState) {
    state.agent_running = false;
    state.current_model = None;
    // BG-5 FIX: Clear any pending permission modal
    if state.mode == TuiMode::Permission {
        state.permission_modal.tool = None;
        state.permission_modal.tool_call_id = None;
    }
    // BG-1 FIX: Clear pending permission queue when agent ends
    state.permission_modal.pending_queue.clear();
    state.mode = TuiMode::Chat;
}

// P1-1 FIX: Sanitize and truncate error messages to prevent raw stack traces
pub fn on_agent_error(state: &mut AppState, message: String) {
    // P1-1: Sanitize error message - truncate long messages and detect stack traces
    let sanitized_message = sanitize_error_message(&message);
    let recoverable = is_recoverable_error(&sanitized_message);
    state.messages.push(MessageItem::Error { message: sanitized_message, recoverable });
    state.agent_running = false;
    // BG-2 FIX: Always reset to Chat on error (unless in Onboarding)
    // Prevents getting stuck in Permission mode if agent errors out
    if state.mode != TuiMode::Onboarding {
        state.mode = TuiMode::Chat;
    }
}

/// P1-1 FIX: Sanitize error messages by truncating long messages and detecting stack traces
pub(crate) fn sanitize_error_message(message: &str) -> String {
    const MAX_ERROR_LENGTH: usize = 500;
    const STACK_TRACE_PATTERNS: &[&str] = &[
        "stack backtrace",
        "thread '",
        "at 0x",
        "panicked at",
        "---- ",
        "FAILED",
        "test result:",
    ];
    
    let message_lower = message.to_lowercase();
    
    // Check if message contains stack trace indicators
    let has_stack_trace = STACK_TRACE_PATTERNS.iter()
        .any(|p| message_lower.contains(&p.to_lowercase()));
    
    if has_stack_trace {
        // Extract just the first line(s) for stack traces - the error summary
        let lines: Vec<&str> = message.lines()
            .take(5)  // Take first 5 lines as summary
            .collect();
        
        let summary = lines.join("\n");
        if summary.len() > MAX_ERROR_LENGTH {
            format!("{}... [truncated - {} chars total]", 
                &summary[..MAX_ERROR_LENGTH.saturating_sub(30)],
                message.len())
        } else {
            format!("{}\n[Additional details hidden. Run with --verbose for full output.]", summary)
        }
    } else if message.len() > MAX_ERROR_LENGTH {
        format!("{}... [message truncated, {} chars total]", 
            &message[..MAX_ERROR_LENGTH.saturating_sub(25)],
            message.len())
    } else {
        message.to_string()
    }
}

/// Classify errors as recoverable or fatal
fn is_recoverable_error(message: &str) -> bool {
    // Transient/network errors are typically recoverable
    let recoverable_patterns = [
        "timeout",
        "connection refused",
        "network",
        "temporary",
        "rate limit",
        "too many requests",
    ];
    let message_lower = message.to_lowercase();
    recoverable_patterns.iter().any(|p| message_lower.contains(p))
}

/// Modes that block permission requests from interrupting
fn is_blocking_mode(mode: &TuiMode) -> bool {
    matches!(mode, TuiMode::Overlay | TuiMode::DiffViewer | TuiMode::SessionTree)
}

pub fn on_permission_request(state: &mut AppState, tool_call_id: String, tool_name: String, tool_args: String) {
    // BG-1 FIX: Queue permission if already in a blocking mode
    if is_blocking_mode(&state.mode) {
        state.permission_modal.pending_queue.push(PendingPermission {
            tool_call_id: tool_call_id.clone(),
            tool_name: tool_name.clone(),
            tool_args: tool_args.clone(),
        });
        // Notify user that request is queued
        state.messages.push(MessageItem::System {
            text: format!("Permission for '{}' queued (waiting for current modal)", tool_name),
        });
        return;
    }
    
    // If permission modal is already open, queue the new request
    if state.mode == TuiMode::Permission || state.permission_modal.tool.is_some() {
        state.permission_modal.pending_queue.push(PendingPermission {
            tool_call_id: tool_call_id.clone(),
            tool_name: tool_name.clone(),
            tool_args: tool_args.clone(),
        });
        return;
    }
    
    state.permission_modal.tool = Some(tool_name.clone());
    state.permission_modal.tool_call_id = Some(tool_call_id);
    state.permission_modal.args = Some(tool_args.clone());
    state.permission_modal.desc = Some(format!("Agent wants to execute '{}'", tool_name));
    // P0-1 FIX: Start timeout tracking
    state.permission_modal.timeout_start = Some(std::time::Instant::now());
    state.permission_modal.timed_out = false;
    state.mode = TuiMode::Permission;
}

/// Process the next pending permission request from the queue
pub fn process_pending_permission(state: &mut AppState) -> Option<PendingPermission> {
    state.permission_modal.pending_queue.pop()
}

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

pub fn to_agent_messages(items: &[MessageItem]) -> Vec<AgentMessage> {
    items.iter().filter_map(|item| match item {
        MessageItem::User { text, .. } => Some(AgentMessage {
            role: "user".to_string(),
            content: vec![ContentPart::Text { text: text.clone() }],
            timestamp: 0,
            usage: None,
            stop_reason: None,
            error_message: None,
        }),
        MessageItem::Assistant { text, .. } => Some(AgentMessage {
            role: "assistant".to_string(),
            content: vec![ContentPart::Text { text: text.clone() }],
            timestamp: 0,
            usage: None,
            stop_reason: None,
            error_message: None,
        }),
        // P2-1: Don't include Error messages in agent context
        MessageItem::Error { .. } => None,
        _ => None,
    }).collect()
}

pub fn handle_permission(state: &mut AppState, decision: PermissionDecision) -> Vec<AgentCmd> {
    let tool_call_id = state.permission_modal.tool_call_id.clone();
    state.permission_modal.tool = None;
    state.permission_modal.tool_call_id = None;

    // P1-4 FIX: On cancel, trigger rollback for the tool that was pending
    let should_rollback = tool_call_id.is_some()
        && (matches!(decision, PermissionDecision::Deny { .. })
            || matches!(decision, PermissionDecision::Skip { .. }));

    let mut cmds = vec![AgentCmd::SendPermission { decision }];
    if should_rollback {
        cmds.push(AgentCmd::Rollback { tool_call_id: tool_call_id.unwrap() });
    }

    // BG-1 FIX: Process next pending permission if any
    if let Some(pending) = state.permission_modal.pending_queue.pop() {
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

    cmds
}

pub fn handle_permission_msg(state: &mut AppState, msg: Msg) -> Vec<AgentCmd> {
    let tool_call_id = state.permission_modal.tool_call_id.clone().unwrap_or_default();
    let tool_name = state.permission_modal.tool.clone().unwrap_or_default();
    let tool_args = state.permission_modal.args.clone().unwrap_or_default();
    let decision = match msg {
        Msg::PermissionConfirm => PermissionDecision::Allow { tool_call_id, tool_name, tool_args },
        Msg::PermissionCancel => PermissionDecision::Deny { tool_call_id, tool_name, tool_args },
        Msg::PermissionAlways => PermissionDecision::AllowAlways { tool_call_id, tool_name, tool_args },
        Msg::PermissionSkip => PermissionDecision::Skip { tool_call_id, tool_name, tool_args },
        _ => PermissionDecision::Allow { tool_call_id, tool_name, tool_args },
    };
    handle_permission(state, decision)
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
    if let Some(pending) = state.permission_modal.pending_queue.pop() {
        // Show the queued permission immediately
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
    
    // P2-5 FIX: Send denial decision to agent loop so it doesn't wait indefinitely
    let tool_name = state.permission_modal.tool.clone().unwrap_or_default();
    let tool_args = state.permission_modal.args.clone().unwrap_or_default();
    vec![Cmd::SendPermission { decision: PermissionDecision::Deny { tool_call_id, tool_name, tool_args } }]
}
