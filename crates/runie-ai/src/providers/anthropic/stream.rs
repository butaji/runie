//! Anthropic streaming implementation.

use async_stream::stream;
use chrono::Utc;
use futures::stream::BoxStream;
use futures::StreamExt;
use reqwest::Response;
use runie_core::{Event, ProviderError};

use super::types::AnthropicStreamChunk;

fn anthropic_stream_init_events(session_id: &str) -> Vec<Event> {
    vec![
        Event::AgentStart { session_id: session_id.to_string(), timestamp: Utc::now() },
        Event::TurnStart { turn: 0, timestamp: Utc::now() },
        Event::MessageStart { role: "assistant".to_string(), timestamp: Utc::now() },
    ]
}

async fn anthropic_stream_process_response(response: Response) -> Vec<Event> {
    let mut events = Vec::new();
    let mut text_content = String::new();
    let mut current_tool_name = String::new();
    let mut current_tool_args = String::new();
    let mut current_tool_id = String::new();
    let mut in_tool_block = false;

    let mut stream = response.bytes_stream();

    while let Some(item) = stream.next().await {
        match item {
            Ok(bytes) => {
                let text = String::from_utf8_lossy(&bytes);
                for line in text.lines() {
                    for event in process_anthropic_line(
                        line,
                        &mut text_content,
                        &mut current_tool_name,
                        &mut current_tool_args,
                        &mut current_tool_id,
                        &mut in_tool_block,
                    ) {
                        events.push(event);
                    }
                }
            }
            Err(e) => {
                events.push(Event::Error { message: e.to_string() });
                break;
            }
        }
    }
    events
}

pub async fn build_anthropic_stream(response: Response) -> Result<BoxStream<'static, Event>, ProviderError> {
    let session_id = format!("anthropic-{}", Utc::now().timestamp_nanos_opt().unwrap_or(0));
    let init_events = anthropic_stream_init_events(&session_id);
    let process_events = anthropic_stream_process_response(response).await;

    let stream = stream! {
        for event in init_events {
            yield event;
        }
        for event in process_events {
            yield event;
        }
    };

    Ok(Box::pin(stream))
}

fn process_anthropic_line(
    line: &str,
    text_content: &mut String,
    current_tool_name: &mut String,
    current_tool_args: &mut String,
    current_tool_id: &mut String,
    in_tool_block: &mut bool,
) -> Vec<Event> {
    if !line.starts_with("event: ") && !line.starts_with("data: ") { return vec![]; }

    let (event_type, data) = if let Some(stripped) = line.strip_prefix("event: ") {
        (stripped.trim(), None)
    } else if let Some(stripped) = line.strip_prefix("data: ") {
        ("", Some(stripped.trim()))
    } else { return vec![]; };

    if !is_relevant_anthropic_event(event_type) { return vec![]; }

    let data_str = match data {
        Some(s) => s,
        None => return vec![],
    };

    let chunk: AnthropicStreamChunk = match serde_json::from_str(data_str) {
        Ok(c) => c,
        Err(_) => return vec![],
    };

    handle_anthropic_chunk(
        &chunk,
        text_content,
        current_tool_name,
        current_tool_args,
        current_tool_id,
        in_tool_block,
    )
}

fn is_relevant_anthropic_event(event_type: &str) -> bool {
    matches!(event_type,
        "message_start" | "content_block_start" | "content_block_delta"
        | "message_delta" | "message_stop" | "content_block_stop"
    )
}

fn handle_anthropic_chunk(
    chunk: &AnthropicStreamChunk,
    text_content: &mut String,
    current_tool_name: &mut String,
    current_tool_args: &mut String,
    current_tool_id: &mut String,
    in_tool_block: &mut bool,
) -> Vec<Event> {
    match chunk {
        AnthropicStreamChunk::MessageStart(_) => vec![],
        AnthropicStreamChunk::ContentBlockStart(cb) => {
            if cb.type_ == "tool_use" {
                *current_tool_name = cb.name.clone().unwrap_or_default();
                current_tool_args.clear();
                *current_tool_id = format!("call_{}", cb.index);
                *in_tool_block = true;
            }
            vec![]
        }
        AnthropicStreamChunk::ContentBlockDelta(delta) => {
            handle_anthropic_delta(text_content, current_tool_args, current_tool_id, current_tool_name, delta)
        }
        AnthropicStreamChunk::ContentBlockStop => {
            *in_tool_block = false;
            vec![]
        }
        AnthropicStreamChunk::MessageDelta(delta) => {
            if let Some(usage) = &delta.usage {
                vec![Event::Usage {
                    prompt_tokens: usage.input_tokens,
                    completion_tokens: usage.output_tokens,
                    total_tokens: usage.input_tokens + usage.output_tokens,
                }]
            } else {
                vec![]
            }
        }
        AnthropicStreamChunk::MessageStop => {
            vec![Event::MessageEnd, Event::AgentEnd { timestamp: Utc::now() }]
        }
    }
}

fn handle_anthropic_delta(
    text_content: &mut String,
    current_tool_args: &mut String,
    current_tool_id: &str,
    current_tool_name: &str,
    delta: &super::types::ContentBlockDelta,
) -> Vec<Event> {
    match delta.type_.as_str() {
        "text_delta" => {
            if let Some(text) = &delta.text {
                text_content.push_str(text);
                vec![Event::MessageDelta { content: text.clone() }]
            } else {
                vec![]
            }
        }
        "input_json_delta" => {
            if let Some(partial) = &delta.partial_json {
                current_tool_args.push_str(partial);
                vec![Event::ToolCallDelta {
                    id: current_tool_id.to_string(),
                    name: current_tool_name.to_string(),
                    arguments: current_tool_args.clone(),
                }]
            } else {
                vec![]
            }
        }
        _ => vec![],
    }
}
