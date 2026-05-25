use crate::components::MessageItem;
use crate::tui::state::{AppState, Msg, Cmd, TuiMode};
use runie_agent::{AgentEvent, AgentMessage, ContentPart, PermissionDecision};
use runie_ai::TokenUsage;

pub fn handle_agent_event(state: &mut AppState, event: AgentEvent) {
    match event {
        AgentEvent::Message { role, content } => on_message(state, &role, &content),
        AgentEvent::MessageStart { message } => on_message_start(state, message),
        AgentEvent::MessageUpdate { message } => on_message_update(state, message),
        AgentEvent::MessageEnd { message } => on_message_end(state, message),
        AgentEvent::ToolExecutionStart { tool_call_id } => on_tool_start(state, tool_call_id),
        AgentEvent::ToolExecutionEnd { result, .. } => on_tool_end(state, result),
        AgentEvent::AgentEnd { .. } => on_agent_end(state),
        AgentEvent::Error { message } => on_agent_error(state, message),
        AgentEvent::PermissionRequest { tool_call_id, tool_name, tool_args } => on_permission_request(state, tool_call_id, tool_name, tool_args),
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
}

pub fn on_agent_error(state: &mut AppState, message: String) {
    state.messages.push(MessageItem::System { text: format!("Error: {}", message) });
    state.agent_running = false;
}

pub fn on_permission_request(state: &mut AppState, tool_call_id: String, tool_name: String, tool_args: String) {
    state.permission_modal.tool = Some(tool_name.clone());
    state.permission_modal.tool_call_id = Some(tool_call_id);
    state.permission_modal.args = Some(tool_args.clone());
    state.permission_modal.desc = Some(format!("Agent wants to execute '{}'", tool_name));
    state.mode = TuiMode::Permission;
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
        _ => None,
    }).collect()
}

pub fn handle_permission(state: &mut AppState, decision: PermissionDecision) -> Cmd {
    state.mode = TuiMode::Chat;
    state.permission_modal.tool = None;
    Cmd::SendPermission { decision }
}

pub fn handle_permission_msg(state: &mut AppState, msg: Msg) -> Cmd {
    let tool_call_id = state.permission_modal.tool_call_id.clone().unwrap_or_default();
    let decision = match msg {
        Msg::PermissionConfirm => PermissionDecision::Allow { tool_call_id },
        Msg::PermissionCancel => PermissionDecision::Deny { tool_call_id },
        Msg::PermissionAlways => PermissionDecision::AllowAlways { tool_call_id },
        Msg::PermissionSkip => PermissionDecision::Skip { tool_call_id },
        _ => PermissionDecision::Allow { tool_call_id },
    };
    handle_permission(state, decision)
}
