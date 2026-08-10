use crate::types::{
    AgentMessage, AssistantContent, AssistantMessage, AssistantMessageEvent, StopReason, ToolCall,
    ToolResultMessage, Usage, WireMessage,
};

pub(super) fn apply_event(assistant: &mut AssistantMessage, event: AssistantMessageEvent) {
    match event {
        AssistantMessageEvent::Start { .. } => {}
        AssistantMessageEvent::TextStart { .. } | AssistantMessageEvent::TextEnd { .. } => {}
        AssistantMessageEvent::TextDelta { delta, .. } => {
            push_or_append(assistant, AssistantContent::Text { text: delta });
        }
        AssistantMessageEvent::ThinkingStart { .. } => {}
        AssistantMessageEvent::ThinkingEnd { elapsed_ms, .. } => {
            assistant.thinking_elapsed_ms = elapsed_ms;
        }
        AssistantMessageEvent::ThinkingDelta { delta, .. } => {
            push_or_append(assistant, AssistantContent::Thinking { text: delta });
        }
        AssistantMessageEvent::ToolCallStart { partial, .. }
        | AssistantMessageEvent::ToolCallDelta { partial, .. } => {
            merge_assistant_partial(assistant, partial);
        }
        AssistantMessageEvent::ToolCallEnd { tool_call, .. } => {
            upsert_tool_call(assistant, tool_call)
        }
        AssistantMessageEvent::Done {
            stop_reason,
            usage,
            message,
        } => {
            apply_done_event(assistant, stop_reason, usage, message);
        }
        AssistantMessageEvent::Error { reason, error } => {
            apply_error_event(assistant, reason, error)
        }
    }
}

fn apply_done_event(
    assistant: &mut AssistantMessage,
    stop_reason: StopReason,
    usage: Usage,
    message: Option<AssistantMessage>,
) {
    assistant.stop_reason = Some(stop_reason);
    assistant.usage = usage;
    if let Some(terminal) = message {
        *assistant = terminal;
    }
}

fn apply_error_event(
    assistant: &mut AssistantMessage,
    reason: StopReason,
    error: AssistantMessage,
) {
    assistant.stop_reason = Some(reason);
    assistant.error_message = error.error_message;
    assistant.usage = error.usage;
    assistant.raw_stop_reason = error.raw_stop_reason;
}

fn upsert_tool_call(assistant: &mut AssistantMessage, call: ToolCall) {
    if let Some(existing) = assistant
        .content
        .iter_mut()
        .find_map(|content| match content {
            AssistantContent::ToolCall(existing) if existing.id == call.id => Some(existing),
            _ => None,
        })
    {
        *existing = call;
    } else {
        assistant.content.push(AssistantContent::ToolCall(call));
    }
}

fn merge_assistant_partial(assistant: &mut AssistantMessage, partial: AssistantMessage) {
    for content in partial.content {
        match content {
            AssistantContent::ToolCall(call) => upsert_tool_call(assistant, call),
            other => push_or_append(assistant, other),
        }
    }
    assistant.stop_reason = partial.stop_reason;
    assistant.usage = partial.usage;
    assistant.thinking_elapsed_ms = partial.thinking_elapsed_ms;
}

fn push_or_append(assistant: &mut AssistantMessage, content: AssistantContent) {
    match (assistant.content.last_mut(), &content) {
        (Some(AssistantContent::Text { text }), AssistantContent::Text { text: new_text }) => {
            text.push_str(new_text);
            return;
        }
        (
            Some(AssistantContent::Thinking { text }),
            AssistantContent::Thinking { text: new_text },
        ) => {
            text.push_str(new_text);
            return;
        }
        _ => {}
    }
    assistant.content.push(content);
}

pub(super) fn enrich_assistant_partial(
    event: AssistantMessageEvent,
    assistant: &AssistantMessage,
) -> AssistantMessageEvent {
    let partial = assistant.clone();
    enrich_text_event(&event, &partial)
        .or_else(|| enrich_thinking_event(&event, &partial))
        .or_else(|| enrich_tool_event(&event, &partial))
        .unwrap_or(event)
}

fn enrich_text_event(
    event: &AssistantMessageEvent,
    partial: &AssistantMessage,
) -> Option<AssistantMessageEvent> {
    Some(match event {
        AssistantMessageEvent::TextStart { index, .. } => AssistantMessageEvent::TextStart {
            index: *index,
            partial: partial.clone(),
        },
        AssistantMessageEvent::TextDelta { index, delta, .. } => AssistantMessageEvent::TextDelta {
            index: *index,
            delta: delta.clone(),
            partial: partial.clone(),
        },
        AssistantMessageEvent::TextEnd { index, content, .. } => AssistantMessageEvent::TextEnd {
            index: *index,
            content: content.clone(),
            partial: partial.clone(),
        },
        _ => return None,
    })
}

fn enrich_thinking_event(
    event: &AssistantMessageEvent,
    partial: &AssistantMessage,
) -> Option<AssistantMessageEvent> {
    Some(match event {
        AssistantMessageEvent::ThinkingStart { index, .. } => {
            AssistantMessageEvent::ThinkingStart {
                index: *index,
                partial: partial.clone(),
            }
        }
        AssistantMessageEvent::ThinkingDelta { index, delta, .. } => {
            AssistantMessageEvent::ThinkingDelta {
                index: *index,
                delta: delta.clone(),
                partial: partial.clone(),
            }
        }
        AssistantMessageEvent::ThinkingEnd {
            index,
            content,
            elapsed_ms,
            ..
        } => AssistantMessageEvent::ThinkingEnd {
            index: *index,
            content: content.clone(),
            elapsed_ms: *elapsed_ms,
            partial: partial.clone(),
        },
        _ => return None,
    })
}

fn enrich_tool_event(
    event: &AssistantMessageEvent,
    partial: &AssistantMessage,
) -> Option<AssistantMessageEvent> {
    Some(match event {
        AssistantMessageEvent::ToolCallEnd {
            index, tool_call, ..
        } => AssistantMessageEvent::ToolCallEnd {
            index: *index,
            tool_call: tool_call.clone(),
            partial: partial.clone(),
        },
        AssistantMessageEvent::ToolCallDelta { index, delta, .. } => {
            AssistantMessageEvent::ToolCallDelta {
                index: *index,
                delta: delta.clone(),
                partial: partial.clone(),
            }
        }
        _ => return None,
    })
}

pub(super) fn is_delta_event(event: &AssistantMessageEvent) -> bool {
    matches!(
        event,
        AssistantMessageEvent::TextStart { .. }
            | AssistantMessageEvent::TextDelta { .. }
            | AssistantMessageEvent::TextEnd { .. }
            | AssistantMessageEvent::ThinkingStart { .. }
            | AssistantMessageEvent::ThinkingDelta { .. }
            | AssistantMessageEvent::ThinkingEnd { .. }
            | AssistantMessageEvent::ToolCallStart { .. }
            | AssistantMessageEvent::ToolCallDelta { .. }
            | AssistantMessageEvent::ToolCallEnd { .. }
    )
}

pub(super) fn wire_to_agent(wire: &[WireMessage]) -> Vec<AgentMessage> {
    wire.iter().map(wire_message_to_agent).collect()
}

fn wire_message_to_agent(w: &WireMessage) -> AgentMessage {
    match w {
        WireMessage::User { content, timestamp } => AgentMessage::User(crate::types::UserMessage {
            content: content.clone(),
            timestamp: *timestamp,
        }),
        WireMessage::Assistant {
            content,
            stop_reason,
            model,
            timestamp,
        } => AgentMessage::Assistant(crate::types::AssistantMessage {
            content: content.clone(),
            stop_reason: *stop_reason,
            model: model.clone(),
            timestamp: *timestamp,
            ..Default::default()
        }),
        WireMessage::ToolResult {
            tool_call_id,
            tool_name,
            content,
            details,
            usage,
            added_tool_names,
            is_error,
            timestamp,
        } => AgentMessage::ToolResult(ToolResultMessage {
            tool_call_id: tool_call_id.clone(),
            tool_name: tool_name.clone(),
            content: content.clone(),
            details: details.clone(),
            usage: usage.clone(),
            added_tool_names: added_tool_names.clone(),
            is_error: *is_error,
            timestamp: *timestamp,
        }),
    }
}
