//! OpenAI streaming implementation.

use async_stream::stream;
use chrono::Utc;
use futures::stream::BoxStream;
use futures::StreamExt;
use reqwest::Response;
use runie_core::{Event, Message, ProviderError};

use super::types::{OpenAIStreamChunk, PendingToolCall};
use crate::token_usage::TokenUsage;

fn openai_stream_init_events(session_id: &str) -> Vec<Event> {
    vec![
        Event::AgentStart { session_id: session_id.to_string(), timestamp: Utc::now() },
        Event::TurnStart { turn: 0, timestamp: Utc::now() },
        Event::MessageStart { role: "assistant".to_string(), timestamp: Utc::now() },
    ]
}

async fn openai_stream_process_response(
    response: Response,
) -> (Vec<Event>, Option<TokenUsage>, String) {
    let mut events = Vec::new();
    let mut text_content = String::new();
    let mut usage: Option<TokenUsage> = None;
    let mut pending_tool_calls: std::collections::HashMap<usize, PendingToolCall> = std::collections::HashMap::new();

    let mut stream = response.bytes_stream();

    while let Some(item) = stream.next().await {
        match item {
            Ok(bytes) => {
                let text = String::from_utf8_lossy(&bytes);
                for line in text.lines() {
                    if let Some(event) = parse_openai_line(line, &mut text_content, &mut usage, &mut pending_tool_calls) {
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
    (events, usage, text_content)
}

pub async fn build_openai_stream(response: Response, messages: Vec<Message>) -> Result<BoxStream<'static, Event>, ProviderError> {
    let session_id = format!("openai-{}", Utc::now().timestamp_nanos_opt().unwrap_or(0));
    let prompt_text = collect_prompt_text(&messages);
    let init_events = openai_stream_init_events(&session_id);
    let (process_events, usage, text_content) = openai_stream_process_response(response).await;

    let (prompt_tokens, completion_tokens, total_tokens) = if let Some(u) = usage {
        (u.prompt_tokens, u.completion_tokens, u.total_tokens)
    } else {
        let (pt, ct) = TokenUsage::estimate_from_text(&prompt_text, &text_content);
        (pt, ct, pt + ct)
    };

    let stream = stream! {
        for event in init_events { yield event; }
        for event in process_events { yield event; }
        yield Event::Usage { prompt_tokens, completion_tokens, total_tokens };
        yield Event::MessageEnd;
        yield Event::AgentEnd { timestamp: Utc::now() };
    };

    Ok(Box::pin(stream))
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

fn parse_openai_line(
    line: &str,
    text_content: &mut String,
    usage: &mut Option<TokenUsage>,
    pending_tool_calls: &mut std::collections::HashMap<usize, PendingToolCall>,
) -> Option<Event> {
    if !line.starts_with("data: ") { return None; }
    let data = &line[6..];
    if data.trim() == "[DONE]" { return None; }

    let chunk: OpenAIStreamChunk = match serde_json::from_str(data) {
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
                    process_openai_tool_call_delta(tc_delta, pending_tool_calls);
                }
            }
        }
    }
    None
}

fn process_openai_tool_call_delta(tc_delta: super::types::ToolCallDelta, pending_tool_calls: &mut std::collections::HashMap<usize, PendingToolCall>) {
    let index = tc_delta.index;
    let id = tc_delta.id.clone();
    let name = tc_delta.function.as_ref().and_then(|f| f.name.clone());
    let args = tc_delta.function.and_then(|f| f.arguments).unwrap_or_default();

    // Handle case where we have a real tool call ID
    if let Some(real_id) = &id {
        return handle_tc_with_id(real_id, index, name, args, pending_tool_calls);
    }

    // Handle case without ID - accumulate in pending map
    if name.is_some() || !args.is_empty() {
        let entry = pending_tool_calls.entry(index).or_insert_with(|| PendingToolCall {
            name: name.clone().unwrap_or_default(),
            arguments: String::new(),
        });
        if let Some(n) = &name {
            entry.name = n.clone();
        }
        entry.arguments.push_str(&args);
    }
}

fn handle_tc_with_id(
    real_id: &str,
    index: usize,
    name: Option<String>,
    args: String,
    pending_tool_calls: &mut std::collections::HashMap<usize, PendingToolCall>,
) {
    if let Some(pending) = pending_tool_calls.remove(&index) {
        let merged_args = format!("{}{}", pending.arguments, args);
        let _ = (real_id, name.unwrap_or_else(|| pending.name), merged_args);
    } else if let Some(n) = name {
        let _ = (real_id, n, args);
    }
}
