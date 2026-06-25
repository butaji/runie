//! OpenAI Chat Completions SSE streaming parser.
//!
//! Uses the `ProviderProtocol` trait to handle SSE frames.

use super::protocol::{OpenAiFrame, OpenAiProtocol, OpenAiState};
use super::request::send_openai_request;
use super::OpenAiProvider;
use crate::framing::sse_framing;
use crate::protocol::ProviderProtocol;
use futures::StreamExt;
use runie_core::provider_event::ProviderEvent;
use runie_core::message::ChatMessage;

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

impl From<ToolCallDelta> for runie_core::message::ToolCall {
    fn from(delta: ToolCallDelta) -> Self {
        let args: serde_json::Value = delta
            .arguments
            .as_ref()
            .and_then(|a| serde_json::from_str(a).ok())
            .unwrap_or(serde_json::Value::Null);
        runie_core::message::ToolCall {
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
        let response = match send_openai_request(&provider.client, &provider, &messages).await {
            Ok(r) => r,
            Err(e) => { yield Err(e); return; }
        };

        let protocol = OpenAiProtocol::new();
        let mut state = OpenAiState::default();
        let mut stream = sse_framing(response.bytes_stream());

        while let Some(result) = stream.next().await {
            let line = match result {
                Ok(l) => l,
                Err(e) => { yield Err(anyhow::anyhow!("SSE framing error: {}", e)); break; }
            };
            let frame = match OpenAiFrame::from_line(&line) {
                Some(f) => f, None => continue,
            };
            let is_terminal = protocol.terminal(&frame);
            let (new_state, events) = protocol.step(state, frame);
            state = new_state;
            for event in events { yield Ok(event); }
            if is_terminal { break; }
        }

        for event in protocol.on_halt(state) { yield Ok(event); }
    }
}

pub fn parse_sse_event(line: &str) -> Option<SseEvent> {
    match OpenAiFrame::from_line(line) {
        Some(OpenAiFrame::Chunk(c)) => Some(SseEvent::Chunk(Chunk {
            delta: Delta {
                content: c.delta.content,
                reasoning: c.delta.reasoning,
                tool_calls: c.delta.tool_calls.into_iter().map(|tc| ToolCallDelta {
                    index: tc.index, id: tc.id, name: tc.name, arguments: tc.arguments,
                }).collect(),
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
        if trimmed.is_empty() { continue; }
        match OpenAiFrame::from_line(trimmed) {
            Some(frame) => {
                if protocol.terminal(&frame) {
                    let (_, new_events) = protocol.step(std::mem::take(&mut state), frame);
                    events.extend(new_events); break;
                }
                let (new_state, new_events) = protocol.step(std::mem::take(&mut state), frame);
                state = new_state; events.extend(new_events);
            }
            None => {}
        }
    }
    events.extend(protocol.on_halt(state)); events
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
            if trimmed.is_empty() { continue; }
            match OpenAiFrame::from_line(trimmed) {
                Some(frame) => {
                    if protocol.terminal(&frame) {
                        let (_, events) = protocol.step(std::mem::take(&mut state), frame);
                        all.extend(events); break;
                    }
                    let (new_state, events) = protocol.step(std::mem::take(&mut state), frame);
                    state = new_state; all.extend(events);
                }
                None => {}
            }
        }
        all.extend(protocol.on_halt(state)); all
    }

    #[test]
    fn text_stream_emits_text_start_before_first_delta() {
        let lines = &[
            "data: {\"choices\":[{\"delta\":{\"content\":\"Hello\"}}]}",
            "data: {\"choices\":[{\"delta\":{\"content\":\" World\"}}]}",
            "data: [DONE]",
        ];
        let events = collect_events(lines);
        let first_delta_idx = events.iter().position(|e| matches!(e, ProviderEvent::TextDelta(_)))
            .expect("Should have TextDelta");
        assert!(matches!(&events[0], ProviderEvent::TextStart { id } if id == "text"), "First event should be TextStart");
        let start_idx = events.iter().position(|e| matches!(e, ProviderEvent::TextStart { .. }))
            .expect("Should have TextStart");
        assert!(start_idx < first_delta_idx);
        let text_starts: Vec<_> = events.iter().filter(|e| matches!(e, ProviderEvent::TextStart { id } if id == "text")).collect();
        assert_eq!(text_starts.len(), 1);
        assert!(events.iter().any(|e| matches!(e, ProviderEvent::Finish { .. })), "Should emit Finish");
    }

    #[test]
    fn reasoning_stream_emits_thinking_start_before_first_delta() {
        let lines = &[
            "data: {\"choices\":[{\"delta\":{\"reasoning_content\":\"thinking\"}}]}",
            "data: {\"choices\":[{\"delta\":{\"reasoning_content\":\" more\"}}]}",
            "data: [DONE]",
        ];
        let events = collect_events(lines);
        let first_delta_idx = events.iter().position(|e| matches!(e, ProviderEvent::ThinkingDelta(_)))
            .expect("Should have ThinkingDelta");
        assert!(matches!(&events[0], ProviderEvent::ThinkingStart { id } if id == "reasoning"), "First event should be ThinkingStart");
        let start_idx = events.iter().position(|e| matches!(e, ProviderEvent::ThinkingStart { .. }))
            .expect("Should have ThinkingStart");
        assert!(start_idx < first_delta_idx);
        let thinking_starts: Vec<_> = events.iter().filter(|e| matches!(e, ProviderEvent::ThinkingStart { id } if id == "reasoning")).collect();
        assert_eq!(thinking_starts.len(), 1);
    }
}
