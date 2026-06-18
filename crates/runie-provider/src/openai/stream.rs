//! OpenAI Chat Completions SSE streaming parser.

use super::request::send_openai_request;
use super::OpenAiProvider;
use futures::StreamExt;
use runie_core::llm_event::{LLMEvent, StopReason};
use runie_core::message::ChatMessage;
use std::collections::{BTreeMap, HashSet};

#[derive(Debug, Default)]
pub struct Delta {
    pub content: Option<String>,
    pub reasoning: Option<String>,
    pub tool_calls: Vec<ToolCallDelta>,
}

#[derive(Debug, Default)]
pub struct ToolCallDelta {
    pub index: usize,
    pub id: Option<String>,
    pub name: Option<String>,
    pub arguments: Option<String>,
}

#[derive(Debug, Default)]
pub struct Chunk {
    pub delta: Delta,
    pub finish_reason: Option<String>,
    pub usage: Option<(usize, usize)>,
}

#[derive(Debug)]
pub enum SseEvent {
    Chunk(Chunk),
    Done,
}

#[derive(Debug, Default)]
struct Accumulator {
    id: String,
    name: String,
    arguments: String,
}

#[derive(Debug, Default)]
struct StreamState {
    tools: BTreeMap<usize, Accumulator>,
    started: HashSet<String>,
    ended: HashSet<String>,
}

pub fn openai_stream(
    provider: OpenAiProvider,
    messages: Vec<ChatMessage>,
) -> std::pin::Pin<Box<dyn futures::Stream<Item = anyhow::Result<LLMEvent>> + Send>> {
    Box::pin(openai_event_stream(provider, messages))
}

fn openai_event_stream(
    provider: OpenAiProvider,
    messages: Vec<ChatMessage>,
) -> impl futures::Stream<Item = anyhow::Result<LLMEvent>> + Send {
    async_stream::stream! {
        let client = reqwest::Client::new();
        let response = match send_openai_request(&client, &provider, &messages).await {
            Ok(r) => r,
            Err(e) => {
                yield Err(e);
                return;
            }
        };

        let mut stream = response.bytes_stream();
        let mut buffer = String::new();
        let mut state = StreamState::default();

        while let Some(chunk_result) = stream.next().await {
            match chunk_result {
                Ok(chunk) => {
                    buffer.push_str(&String::from_utf8_lossy(&chunk));
                    for event in drain_buffer(&mut buffer, &mut state) {
                        yield Ok(event);
                    }
                }
                Err(e) => {
                    yield Err(anyhow::anyhow!("SSE stream error: {}", e));
                    return;
                }
            }
        }

        for event in flush_tool_calls(&mut state) {
            yield Ok(event);
        }
    }
}

pub fn parse_sse_event(line: &str) -> Option<SseEvent> {
    let data = line.strip_prefix("data: ")?;
    if data == "[DONE]" {
        return Some(SseEvent::Done);
    }
    let json: serde_json::Value = serde_json::from_str(data).ok()?;
    parse_chunk(&json).map(SseEvent::Chunk)
}

fn parse_chunk(json: &serde_json::Value) -> Option<Chunk> {
    let choice = json.get("choices")?.get(0)?;
    let delta = choice.get("delta")?;

    Some(Chunk {
        delta: Delta {
            content: delta
                .get("content")
                .and_then(|v| v.as_str())
                .map(String::from),
            reasoning: extract_reasoning(delta),
            tool_calls: parse_tool_call_deltas(delta),
        },
        finish_reason: choice
            .get("finish_reason")
            .and_then(|v| v.as_str())
            .map(String::from),
        usage: json.get("usage").and_then(parse_usage),
    })
}

fn parse_usage(value: &serde_json::Value) -> Option<(usize, usize)> {
    Some((
        value
            .get("prompt_tokens")
            .and_then(|v| v.as_u64())
            .unwrap_or(0) as usize,
        value
            .get("completion_tokens")
            .and_then(|v| v.as_u64())
            .unwrap_or(0) as usize,
    ))
}

fn extract_reasoning(delta: &serde_json::Value) -> Option<String> {
    delta
        .get("reasoning_content")
        .or_else(|| delta.get("reasoning"))
        .and_then(|v| v.as_str())
        .map(String::from)
}

fn parse_tool_call_deltas(delta: &serde_json::Value) -> Vec<ToolCallDelta> {
    delta
        .get("tool_calls")
        .and_then(|v| v.as_array())
        .map(|arr| arr.iter().filter_map(parse_tool_call_delta).collect())
        .unwrap_or_default()
}

fn parse_tool_call_delta(value: &serde_json::Value) -> Option<ToolCallDelta> {
    let index = value.get("index").and_then(|v| v.as_u64()).unwrap_or(0) as usize;
    let function = value.get("function").unwrap_or(value);
    Some(ToolCallDelta {
        index,
        id: value.get("id").and_then(|v| v.as_str()).map(String::from),
        name: function
            .get("name")
            .and_then(|v| v.as_str())
            .map(String::from),
        arguments: function
            .get("arguments")
            .and_then(|v| v.as_str())
            .map(String::from),
    })
}

fn drain_buffer(buffer: &mut String, state: &mut StreamState) -> Vec<LLMEvent> {
    let mut events = Vec::new();
    while let Some(pos) = buffer.find('\n') {
        let line = buffer[..pos].trim().to_string();
        *buffer = buffer[pos + 1..].to_string();
        match parse_sse_event(&line) {
            Some(SseEvent::Done) => {
                events.extend(flush_tool_calls(state));
                events.push(LLMEvent::Finish {
                    reason: StopReason::Stop,
                });
                break;
            }
            Some(SseEvent::Chunk(chunk)) => {
                events.extend(process_chunk(chunk, state));
            }
            None => {}
        }
    }
    events
}

fn process_chunk(chunk: Chunk, state: &mut StreamState) -> Vec<LLMEvent> {
    let mut events = Vec::new();

    if let Some(text) = chunk.delta.content {
        events.push(LLMEvent::TextDelta(text));
    }
    if let Some(reasoning) = chunk.delta.reasoning {
        events.push(LLMEvent::ThinkingDelta(reasoning));
    }

    for tool_delta in chunk.delta.tool_calls {
        events.extend(process_tool_call_delta(tool_delta, state));
    }

    if chunk.finish_reason.is_some() {
        events.extend(flush_tool_calls(state));
        events.push(LLMEvent::Finish {
            reason: map_finish_reason(chunk.finish_reason.as_deref()),
        });
    }

    if let Some((input, output)) = chunk.usage {
        events.push(LLMEvent::Usage {
            input_tokens: input,
            output_tokens: output,
        });
    }

    events
}

fn process_tool_call_delta(delta: ToolCallDelta, state: &mut StreamState) -> Vec<LLMEvent> {
    let mut events = Vec::new();
    let acc = state.tools.entry(delta.index).or_default();

    if let Some(id) = delta.id {
        acc.id = id;
    }
    if let Some(name) = delta.name {
        acc.name = name;
    }
    if let Some(args) = delta.arguments {
        if acc.id.is_empty() || acc.name.is_empty() {
            acc.arguments.push_str(&args);
            return events;
        }
        if !state.started.contains(&acc.id) {
            state.started.insert(acc.id.clone());
            events.push(LLMEvent::ToolCallStart {
                id: acc.id.clone(),
                name: acc.name.clone(),
            });
            if !acc.arguments.is_empty() {
                events.push(LLMEvent::ToolCallInputDelta {
                    id: acc.id.clone(),
                    delta: acc.arguments.clone(),
                });
                acc.arguments.clear();
            }
        }
        acc.arguments.push_str(&args);
        events.push(LLMEvent::ToolCallInputDelta {
            id: acc.id.clone(),
            delta: args,
        });
    }

    events
}

fn flush_tool_calls(state: &mut StreamState) -> Vec<LLMEvent> {
    let mut events = Vec::new();
    for acc in state.tools.values() {
        if acc.id.is_empty() || state.ended.contains(&acc.id) {
            continue;
        }
        if !state.started.contains(&acc.id) {
            state.started.insert(acc.id.clone());
            events.push(LLMEvent::ToolCallStart {
                id: acc.id.clone(),
                name: acc.name.clone(),
            });
        }
        if !acc.arguments.is_empty() {
            events.push(LLMEvent::ToolCallInputDelta {
                id: acc.id.clone(),
                delta: acc.arguments.clone(),
            });
        }
        state.ended.insert(acc.id.clone());
        events.push(LLMEvent::ToolCallEnd { id: acc.id.clone() });
    }
    events
}

fn map_finish_reason(reason: Option<&str>) -> StopReason {
    match reason {
        Some("stop") => StopReason::Stop,
        Some("length") => StopReason::Length,
        Some("content_filter") => StopReason::ContentFilter,
        Some("tool_calls") => StopReason::ToolCalls,
        Some("stop_sequence") => StopReason::StopSequence,
        _ => StopReason::Stop,
    }
}

#[cfg(test)]
pub mod tests {
    use super::*;

    pub fn collect_events(lines: &[&str]) -> Vec<LLMEvent> {
        let mut state = StreamState::default();
        let mut all = Vec::new();
        for line in lines {
            match parse_sse_event(line) {
                Some(SseEvent::Chunk(chunk)) => all.extend(process_chunk(chunk, &mut state)),
                Some(SseEvent::Done) => {
                    all.extend(flush_tool_calls(&mut state));
                    all.push(LLMEvent::Finish {
                        reason: StopReason::Stop,
                    });
                }
                None => {}
            }
        }
        all.extend(flush_tool_calls(&mut state));
        all
    }
}
