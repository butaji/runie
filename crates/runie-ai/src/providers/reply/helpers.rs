//! Shared helper functions for ReplyProvider.

use chrono::Utc;
use runie_core::Event;

use super::{RecordedResponse, RecordedToolCall};

/// Create base events for agent start.
pub fn agent_start_events(session_id: &str) -> Vec<Event> {
    vec![
        Event::AgentStart {
            session_id: session_id.to_string(),
            timestamp: Utc::now(),
        },
        Event::TurnStart {
            turn: 0,
            timestamp: Utc::now(),
        },
        Event::MessageStart {
            role: "assistant".to_string(),
            timestamp: Utc::now(),
        },
    ]
}

/// Create base events for agent end.
pub fn agent_end_events() -> Vec<Event> {
    vec![
        Event::MessageEnd,
        Event::AgentEnd {
            timestamp: Utc::now(),
        },
    ]
}

/// Add usage from a response to events.
pub fn add_usage(events: &mut Vec<Event>, response: &RecordedResponse) {
    if let Some(usage) = &response.usage {
        events.push(Event::Usage {
            prompt_tokens: usage.prompt_tokens.unwrap_or(0),
            completion_tokens: usage.completion_tokens.unwrap_or(0),
            total_tokens: usage.total_tokens,
        });
    }
}

/// Extract delta content (reasoning + message) from a response choice.
pub fn extract_delta_content(delta: &RecordedResponse, events: &mut Vec<Event>) {
    if let Some(choices) = &delta.choices {
        if let Some(choice) = choices.first() {
            if let Some(d) = &choice.delta {
                if let Some(reasoning) = &d.reasoning_content {
                    events.push(Event::ThinkingDelta {
                        content: reasoning.clone(),
                    });
                }
                if let Some(c) = &d.content {
                    events.push(Event::MessageDelta {
                        content: c.clone(),
                    });
                }
            }
        }
    }
}

/// Extract tool calls from a delta (streaming).
pub fn extract_delta_tool_calls(delta: &RecordedToolCall) -> Event {
    let id = delta.id.clone().unwrap_or_default();
    let name = delta.function.as_ref()
        .and_then(|f| f.name.clone())
        .unwrap_or_default();
    let args = delta.function.as_ref()
        .and_then(|f| f.arguments.clone())
        .unwrap_or_default();
    Event::ToolCallDelta { id, name, arguments: args }
}

/// Convert a RecordedToolCall to an Event.
pub fn tool_call_to_event(tc: &RecordedToolCall) -> Event {
    let id = tc.id.clone().unwrap_or_default();
    let name = tc.function.as_ref()
        .and_then(|f| f.name.clone())
        .unwrap_or_default();
    let args = tc.function.as_ref()
        .and_then(|f| f.arguments.clone())
        .unwrap_or_default();
    Event::ToolCallDelta { id, name, arguments: args }
}

/// Append usage from SSE chunks to events.
pub fn append_usage_from_chunks(events: &mut Vec<Event>, chunks: &[String]) {
    for chunk_json in chunks {
        if let Ok(chunk) = serde_json::from_str::<RecordedResponse>(chunk_json) {
            if let Some(usage) = &chunk.usage {
                if usage.total_tokens > 0 {
                    events.push(Event::Usage {
                        prompt_tokens: usage.prompt_tokens.unwrap_or(0),
                        completion_tokens: usage.completion_tokens.unwrap_or(0),
                        total_tokens: usage.total_tokens,
                    });
                    break;
                }
            }
        }
    }
}

/// Format error message from error response.
pub fn format_error_message(response: &RecordedResponse) -> String {
    format!(
        "MiniMax API error: status_code={}, status_msg={}",
        response.base_resp.as_ref()
            .map(|b| b.status_code)
            .unwrap_or(0),
        response.base_resp.as_ref()
            .map(|b| b.status_msg.clone())
            .unwrap_or_default()
    )
}

/// Extract message content from events.
pub fn extract_content_from_events(events: &[Event]) -> String {
    let mut content = String::new();
    for event in events {
        if let Event::MessageDelta { content: c } = event {
            content.push_str(c);
        }
    }
    content
}
