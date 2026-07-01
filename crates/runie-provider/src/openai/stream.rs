//! OpenAI Chat Completions SSE streaming parser.
//!
//! Uses the `ProviderProtocol` trait to handle SSE frames.

use super::protocol::{OpenAiFrame, OpenAiProtocol, OpenAiState};
use super::request::build_request_body;
use super::OpenAiProvider;
use crate::protocol::ProviderProtocol;
use reqwest_eventsource::retry::ExponentialBackoff;
use reqwest_eventsource::EventSource;
use runie_core::proto::message::ChatMessage;
use runie_core::provider_event::ProviderEvent;

/// Re-export types for testing and external consumers.
pub use super::protocol::ToolAccum;

/// OpenAI SSE event types.
#[derive(Debug, Clone)]
pub enum SseEvent {
    Chunk(Chunk),
    Done,
}

/// A delta of content in an OpenAI chunk.
#[derive(Debug, Default, Clone, PartialEq)]
pub struct Delta {
    pub content: Option<String>,
    pub reasoning: Option<String>,
    pub tool_calls: Vec<ToolCallDelta>,
}

/// A delta for a tool call.
#[derive(Debug, Default, Clone, PartialEq)]
pub struct ToolCallDelta {
    pub index: usize,
    pub id: Option<String>,
    pub name: Option<String>,
    pub arguments: Option<String>,
}

impl From<ToolCallDelta> for runie_core::proto::message::ToolCall {
    fn from(delta: ToolCallDelta) -> Self {
        let args: serde_json::Value = delta
            .arguments
            .as_ref()
            .and_then(|a| serde_json::from_str(a).ok())
            .unwrap_or(serde_json::Value::Null);
        runie_core::proto::message::ToolCall {
            id: delta.id.unwrap_or_default(),
            name: delta.name.unwrap_or_default(),
            args,
        }
    }
}

/// An OpenAI SSE chunk.
#[derive(Debug, Default, Clone, PartialEq)]
pub struct Chunk {
    pub delta: Delta,
    pub finish_reason: Option<String>,
    pub usage: Option<(usize, usize)>,
}

pub fn openai_stream(
    provider: OpenAiProvider,
    messages: Vec<ChatMessage>,
) -> std::pin::Pin<Box<dyn futures::Stream<Item = anyhow::Result<ProviderEvent>> + Send>> {
    Box::pin(openai_event_stream(provider, messages))
}

fn openai_event_stream(
    provider: OpenAiProvider,
    messages: Vec<ChatMessage>,
) -> impl futures::Stream<Item = anyhow::Result<ProviderEvent>> + Send {
    async_stream::stream! {
        let mut es = match build_eventsource(&provider, &messages) {
            Ok(es) => es,
            Err(e) => { yield Err(e); return; }
        };
        configure_backoff(&mut es);
        let mut es = Box::pin(es);
        let state = stream_sse_events(&mut es).await;
        for event in OpenAiProtocol::new().on_halt(state) { yield Ok(event); }
    }
}

/// Configure exponential backoff with max 3 retries for transient errors.
fn configure_backoff(es: &mut EventSource) {
    let backoff = ExponentialBackoff::new(
        std::time::Duration::from_millis(500),
        2.0,
        Some(std::time::Duration::from_secs(10)),
        Some(3),
    );
    es.set_retry_policy(Box::new(backoff));
}

/// Process SSE events and yield provider events.
async fn stream_sse_events(
    es: &mut (impl futures::Stream<Item = Result<reqwest_eventsource::Event, reqwest_eventsource::Error>>
              + Unpin),
) -> OpenAiState {
    let protocol = OpenAiProtocol::new();
    let mut state = OpenAiState::default();

    while let Some(result) = futures::StreamExt::next(&mut *es).await {
        let event = match parse_sse_result(result) {
            Some(Ok(e)) => e,
            Some(Err(_)) => return state,
            None => continue,
        };
        let frame = match OpenAiFrame::from_line(&event) {
            Some(f) => f,
            None => continue,
        };
        let is_terminal = protocol.terminal(&frame);
        let (new_state, _) = protocol.step(state, frame);
        state = new_state;
        if is_terminal {
            break;
        }
    }
    state
}

fn build_eventsource(
    provider: &OpenAiProvider,
    messages: &[ChatMessage],
) -> anyhow::Result<reqwest_eventsource::EventSource> {
    let body = build_request_body(provider, messages);
    let url = format!("{}/chat/completions", provider.base_url);
    let api_key = &provider.api_key;

    EventSource::new(
        provider
            .client
            .post(&url)
            .header("Authorization", format!("Bearer {}", api_key.trim()))
            .header("Content-Type", "application/json")
            .json(&body),
    )
    .map_err(|e| anyhow::anyhow!("reqwest-eventsource: {}", e))
}

fn parse_sse_result(
    result: Result<reqwest_eventsource::Event, reqwest_eventsource::Error>,
) -> Option<anyhow::Result<String>> {
    match result {
        Ok(reqwest_eventsource::Event::Open) => None,
        Ok(reqwest_eventsource::Event::Message(msg)) => Some(Ok(msg.data)),
        Err(e) => Some(Err(anyhow::anyhow!("SSE stream error: {}", e))),
    }
}

pub fn parse_sse_event(line: &str) -> Option<SseEvent> {
    match OpenAiFrame::from_line(line) {
        Some(OpenAiFrame::Chunk(c)) => Some(SseEvent::Chunk(Chunk {
            delta: Delta {
                content: c.delta.content,
                reasoning: c.delta.reasoning,
                tool_calls: c
                    .delta
                    .tool_calls
                    .into_iter()
                    .map(|tc| ToolCallDelta {
                        index: tc.index,
                        id: tc.id,
                        name: tc.name,
                        arguments: tc.arguments,
                    })
                    .collect(),
            },
            finish_reason: c.finish_reason,
            usage: c.usage,
        })),
        Some(OpenAiFrame::Done) => Some(SseEvent::Done),
        None => None,
    }
}

/// Replay SSE text and return accumulated events.
pub fn replay_sse(text: &str) -> Vec<ProviderEvent> {
    let protocol = OpenAiProtocol::new();
    let mut state = OpenAiState::default();
    let mut events = Vec::new();

    for line in text.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        // Support fixture error lines: "error: {\"type\":\"rate_limit\",...}"
        if let Some(err_json) = trimmed.strip_prefix("error: ") {
            if let Ok(err_val) = serde_json::from_str::<serde_json::Value>(err_json) {
                let model_err = parse_error_value(&err_val);
                events.push(ProviderEvent::Error(model_err));
            }
            continue;
        }
        if let Some(frame) = OpenAiFrame::from_line(trimmed) {
            if protocol.terminal(&frame) {
                let (_, new_events) = protocol.step(std::mem::take(&mut state), frame);
                events.extend(new_events);
                break;
            }
            let (new_state, new_events) = protocol.step(std::mem::take(&mut state), frame);
            state = new_state;
            events.extend(new_events);
        }
    }
    events.extend(protocol.on_halt(state));
    events
}

/// Parse an SSE error line value into a ModelError.
fn parse_error_value(val: &serde_json::Value) -> runie_core::provider_event::ModelError {
    use runie_core::provider_event::ModelError;
    let msg = val
        .get("message")
        .and_then(|v| v.as_str())
        .unwrap_or("unknown error");
    let code = val
        .get("code")
        .and_then(|v| v.as_str())
        .unwrap_or("");
    if code.contains("rate_limit") || val
        .get("type")
        .and_then(|v| v.as_str())
        .map(|t| t.contains("rate_limit"))
        .unwrap_or(false)
    {
        let retry_after = val
            .get("retry_after")
            .and_then(|v| v.as_u64())
            .map(|v| v as u32);
        return ModelError::RateLimit { retry_after_secs: retry_after };
    }
    if code.contains("context_length") || code.contains("token_limit") {
        let limit = val
            .get("limit")
            .and_then(|v| v.as_u64())
            .map(|v| v as usize)
            .unwrap_or(0);
        let used = val
            .get("used")
            .and_then(|v| v.as_u64())
            .map(|v| v as usize)
            .unwrap_or(0);
        return ModelError::ContextLength { limit, used };
    }
    if code.contains("content_filter") || code.contains("refusal") {
        return ModelError::Refusal(msg.to_string());
    }
    ModelError::Other(msg.to_string())
}

#[cfg(test)]
pub mod tests {
    use super::*;

    pub fn collect_events(lines: &[&str]) -> Vec<ProviderEvent> {
        let protocol = OpenAiProtocol::new();
        let mut state = OpenAiState::default();
        let mut all = Vec::new();
        for line in lines {
            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue;
            }
            if let Some(frame) = OpenAiFrame::from_line(trimmed) {
                if protocol.terminal(&frame) {
                    let (_, events) = protocol.step(std::mem::take(&mut state), frame);
                    all.extend(events);
                    break;
                }
                let (new_state, events) = protocol.step(std::mem::take(&mut state), frame);
                state = new_state;
                all.extend(events);
            }
        }
        all.extend(protocol.on_halt(state));
        all
    }

    #[test]
    fn text_stream_emits_text_start_before_first_delta() {
        let lines = &[
            "data: {\"choices\":[{\"delta\":{\"content\":\"Hello\"}}]}",
            "data: {\"choices\":[{\"delta\":{\"content\":\" World\"}}]}",
            "data: [DONE]",
        ];
        let events = collect_events(lines);
        let first_delta_idx = events
            .iter()
            .position(|e| matches!(e, ProviderEvent::TextDelta(_)))
            .expect("Should have TextDelta");
        assert!(
            matches!(&events[0], ProviderEvent::TextStart { id } if id == "text"),
            "First event should be TextStart"
        );
        let start_idx = events
            .iter()
            .position(|e| matches!(e, ProviderEvent::TextStart { .. }))
            .expect("Should have TextStart");
        assert!(start_idx < first_delta_idx);
        let text_starts: Vec<_> = events
            .iter()
            .filter(|e| matches!(e, ProviderEvent::TextStart { id } if id == "text"))
            .collect();
        assert_eq!(text_starts.len(), 1);
        assert!(
            events
                .iter()
                .any(|e| matches!(e, ProviderEvent::Finish { .. })),
            "Should emit Finish"
        );
    }

    #[test]
    fn reasoning_stream_emits_thinking_start_before_first_delta() {
        let lines = &[
            "data: {\"choices\":[{\"delta\":{\"reasoning_content\":\"thinking\"}}]}",
            "data: {\"choices\":[{\"delta\":{\"reasoning_content\":\" more\"}}]}",
            "data: [DONE]",
        ];
        let events = collect_events(lines);
        let first_delta_idx = events
            .iter()
            .position(|e| matches!(e, ProviderEvent::ThinkingDelta(_)))
            .expect("Should have ThinkingDelta");
        assert!(
            matches!(&events[0], ProviderEvent::ThinkingStart { id } if id == "reasoning"),
            "First event should be ThinkingStart"
        );
        let start_idx = events
            .iter()
            .position(|e| matches!(e, ProviderEvent::ThinkingStart { .. }))
            .expect("Should have ThinkingStart");
        assert!(start_idx < first_delta_idx);
        let thinking_starts: Vec<_> = events
            .iter()
            .filter(|e| matches!(e, ProviderEvent::ThinkingStart { id } if id == "reasoning"))
            .collect();
        assert_eq!(thinking_starts.len(), 1);
    }

    /// Layer 4: OpenAI SSE stream accumulates to canonical ToolCall.
    #[test]
    fn openai_stream_accumulates_canonical_tool_calls() {
        let lines = &[
            r#"data: {"choices":[{"delta":{"tool_calls":[{"index":0,"id":"call_abc","type":"function","function":{"name":"read_file","arguments":""}}]}}]}"#,
            r#"data: {"choices":[{"delta":{"tool_calls":[{"index":0,"function":{"arguments":"{\"path\":\"C"}}]}}]}"#,
            r#"data: {"choices":[{"delta":{"tool_calls":[{"index":0,"function":{"arguments":"argo.toml\"}"}}]}}]}"#,
            "data: [DONE]",
        ];
        let events = collect_events(lines);
        // Find ToolCallEnd events and verify they contain canonical ToolCalls
        let tool_ends: Vec<_> = events
            .iter()
            .filter_map(|e| {
                if let ProviderEvent::ToolCallEnd { id } = e {
                    Some(id.clone())
                } else {
                    None
                }
            })
            .collect();
        assert_eq!(
            tool_ends,
            vec!["call_abc"],
            "Should emit ToolCallEnd with canonical id"
        );
        // Verify ToolCallStart was emitted with the name
        let tool_starts: Vec<_> = events
            .iter()
            .filter_map(|e| {
                if let ProviderEvent::ToolCallStart { id, name } = e {
                    Some((id.clone(), name.clone()))
                } else {
                    None
                }
            })
            .collect();
        assert_eq!(tool_starts, vec![("call_abc".into(), "read_file".into())]);
    }
}
