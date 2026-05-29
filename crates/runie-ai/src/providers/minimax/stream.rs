//! MiniMax streaming implementation.

use async_stream::stream;
use chrono::Utc;
use futures::stream::BoxStream;
use futures::StreamExt;
use reqwest::Response;
use runie_core::{Event, Message, ProviderError};

use super::types::{MiniMaxStreamChunk, PendingToolCall};
use crate::token_usage::TokenUsage;

pub async fn build_minimax_stream(
    response: Response,
    messages: Vec<Message>,
) -> Result<BoxStream<'static, Event>, ProviderError> {
    let session_id = format!("minimax-{}", Utc::now().timestamp_nanos_opt().unwrap_or(0));
    let prompt_text = collect_prompt_text(&messages);

    let stream = stream! {
        yield agent_start_event(&session_id);
        yield turn_start_event();
        yield message_start_event();

        let mut state = StreamState::new();
        let mut stream = response.bytes_stream();

        while let Some(item) = stream.next().await {
            match item {
                Ok(bytes) => {
                    for event in process_bytes(&mut state, bytes) {
                        yield event;
                    }
                }
                Err(e) => {
                    yield Event::Error { message: e.to_string() };
                    break;
                }
            }
        }

        let usage = finalize_usage(state.usage, &prompt_text, &state.text_content);
        yield Event::Usage { prompt_tokens: usage.0, completion_tokens: usage.1, total_tokens: usage.2 };
        yield Event::MessageEnd;
        yield Event::AgentEnd { timestamp: Utc::now() };
    };

    Ok(Box::pin(stream))
}

struct StreamState {
    text_content: String,
    usage: Option<TokenUsage>,
    pending_tool_calls: std::collections::HashMap<usize, PendingToolCall>,
}

impl StreamState {
    fn new() -> Self {
        Self {
            text_content: String::new(),
            usage: None,
            pending_tool_calls: std::collections::HashMap::new(),
        }
    }
}

fn process_bytes(state: &mut StreamState, bytes: bytes::Bytes) -> Vec<Event> {
    let mut events = Vec::new();
    let text = String::from_utf8_lossy(&bytes);
    for line in text.lines() {
        if let Some(event) = parse_minimax_line(
            line,
            &mut state.text_content,
            &mut state.usage,
            &mut state.pending_tool_calls,
        ) {
            events.push(event);
        }
    }
    events
}

fn finalize_usage(
    usage: Option<TokenUsage>,
    prompt_text: &str,
    text_content: &str,
) -> (usize, usize, usize) {
    if let Some(u) = usage {
        (u.prompt_tokens, u.completion_tokens, u.total_tokens)
    } else {
        let (pt, ct) = TokenUsage::estimate_from_text(prompt_text, text_content);
        (pt, ct, pt + ct)
    }
}

fn agent_start_event(session_id: &str) -> Event {
    Event::AgentStart { session_id: session_id.to_string(), timestamp: Utc::now() }
}

fn turn_start_event() -> Event {
    Event::TurnStart { turn: 0, timestamp: Utc::now() }
}

fn message_start_event() -> Event {
    Event::MessageStart { role: "assistant".to_string(), timestamp: Utc::now() }
}

fn collect_prompt_text(messages: &[Message]) -> String {
    let mut prompt_text = String::new();
    for msg in messages {
        match msg {
            Message::System { content } => prompt_text.push_str(content),
            Message::User { content, .. } => prompt_text.push_str(content),
            Message::Assistant { content, .. } => prompt_text.push_str(content),
            Message::ToolResult { content, .. } => prompt_text.push_str(content),
        }
    }
    prompt_text
}

fn parse_minimax_line(
    line: &str,
    text_content: &mut String,
    usage: &mut Option<TokenUsage>,
    pending_tool_calls: &mut std::collections::HashMap<usize, PendingToolCall>,
) -> Option<Event> {
    if !line.starts_with("data: ") { return None; }
    let data = &line[6..];
    if data.trim() == "[DONE]" { return None; }

    let chunk: MiniMaxStreamChunk = match serde_json::from_str(data) {
        Ok(c) => c,
        Err(_) => return None,
    };

    if chunk.usage.is_some() { *usage = chunk.usage; }
    if let Some(choice) = chunk.choices.into_iter().next() {
        if let Some(delta) = choice.delta {
            if let Some(c) = delta.content {
                text_content.push_str(&c);
                return Some(Event::MessageDelta { content: c });
            }
            if let Some(tool_calls) = delta.tool_calls {
                for tc_delta in tool_calls {
                    process_minimax_tool_call_delta(tc_delta, pending_tool_calls, text_content);
                }
            }
        }
    }
    None
}

fn process_minimax_tool_call_delta(
    tc_delta: super::types::MiniMaxToolCallDelta,
    pending_tool_calls: &mut std::collections::HashMap<usize, PendingToolCall>,
    _text_content: &mut String,
) {
    let index = tc_delta.index;
    let id = tc_delta.id.clone();
    let function = tc_delta.function.clone();
    let name = function.as_ref().and_then(|f| f.name.clone());
    let args = function.and_then(|f| f.arguments).unwrap_or_default();

    if let Some(real_id) = &id {
        if let Some(pending) = pending_tool_calls.remove(&index) {
            let merged_args = format!("{}{}", pending.arguments, args);
            let _ = (real_id, pending.name, merged_args);
        } else if let Some(n) = name {
            let _ = (real_id, n, args);
        }
    } else if name.is_some() || !args.is_empty() {
        let entry = pending_tool_calls.entry(index).or_insert_with(|| PendingToolCall {
            name: name.clone().unwrap_or_default(),
            arguments: String::new(),
        });
        if let Some(n) = &name { entry.name = n.clone(); }
        entry.arguments.push_str(&args);
    }
}
