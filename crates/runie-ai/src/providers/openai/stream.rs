//! OpenAI streaming implementation.

use async_stream::stream;
use chrono::Utc;
use futures::stream::BoxStream;
use futures::StreamExt;
use reqwest::Response;
use runie_core::{Event, Message, ProviderError};

use super::types::{OpenAIStreamChunk, PendingToolCall};
use crate::token_usage::TokenUsage;

pub async fn build_openai_stream(response: Response, messages: Vec<Message>) -> Result<BoxStream<'static, Event>, ProviderError> {
    let session_id = format!("openai-{}", Utc::now().timestamp_nanos_opt().unwrap_or(0));

    let stream = stream! {
        yield Event::AgentStart { session_id: session_id.clone(), timestamp: Utc::now() };
        yield Event::TurnStart { turn: 0, timestamp: Utc::now() };
        yield Event::MessageStart { role: "assistant".to_string(), timestamp: Utc::now() };

        let mut text_content = String::new();
        let mut prompt_text = String::new();
        for msg in &messages {
            match msg {
                Message::System { content } => prompt_text.push_str(content),
                Message::User { content, .. } => prompt_text.push_str(content),
                Message::Assistant { content, .. } => prompt_text.push_str(content),
                Message::ToolResult { content, .. } => prompt_text.push_str(content),
            }
        }

        let mut usage: Option<TokenUsage> = None;
        let mut stream = response.bytes_stream();
        let mut pending_tool_calls: std::collections::HashMap<usize, PendingToolCall> = std::collections::HashMap::new();

        while let Some(item) = stream.next().await {
            match item {
                Ok(bytes) => {
                    let text = String::from_utf8_lossy(&bytes);
                    for line in text.lines() {
                        if !line.starts_with("data: ") { continue; }
                        let data = &line[6..];
                        if data.trim() == "[DONE]" { continue; }
                        let chunk: OpenAIStreamChunk = match serde_json::from_str(data) { Ok(c) => c, Err(_) => continue };
                        if chunk.usage.is_some() { usage = chunk.usage; }
                        if let Some(choice) = chunk.choices.into_iter().next() {
                            if let Some(delta) = choice.delta {
                                if let Some(c) = delta.content {
                                    text_content.push_str(&c);
                                    yield Event::MessageDelta { content: c };
                                }
                                if let Some(tool_calls) = delta.tool_calls {
                                    for tc_delta in tool_calls { process_tool_call_delta(tc_delta, &mut pending_tool_calls); }
                                }
                            }
                        }
                    }
                }
                Err(e) => { yield Event::Error { message: e.to_string() }; break; }
            }
        }

        let (prompt_tokens, completion_tokens, total_tokens) = if let Some(u) = usage { (u.prompt_tokens, u.completion_tokens, u.total_tokens) }
            else { let (pt, ct) = TokenUsage::estimate_from_text(&prompt_text, &text_content); (pt, ct, pt + ct) };
        yield Event::Usage { prompt_tokens, completion_tokens, total_tokens };
        yield Event::MessageEnd;
        yield Event::AgentEnd { timestamp: Utc::now() };
    };

    Ok(Box::pin(stream))
}

fn process_tool_call_delta(tc_delta: super::types::ToolCallDelta, pending_tool_calls: &mut std::collections::HashMap<usize, PendingToolCall>) {
    let index = tc_delta.index;
    let id = tc_delta.id.clone();
    let name = tc_delta.function.as_ref().and_then(|f| f.name.clone());
    let args = tc_delta.function.and_then(|f| f.arguments).unwrap_or_default();

    if let Some(real_id) = &id {
        if let Some(pending) = pending_tool_calls.remove(&index) {
            let merged_args = format!("{}{}", pending.arguments, args);
            let _ = (real_id, name.unwrap_or_else(|| pending.name), merged_args);
        } else if let Some(n) = name {
            let _ = (real_id, n, args);
        }
    } else if name.is_some() || !args.is_empty() {
        let entry = pending_tool_calls.entry(index).or_insert_with(|| PendingToolCall { name: name.clone().unwrap_or_default(), arguments: String::new() });
        if let Some(n) = &name { entry.name = n.clone(); }
        entry.arguments.push_str(&args);
    }
}
