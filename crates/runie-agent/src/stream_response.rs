//! Stream response handling for the agent loop.
//!
//! Normalizes provider-native tool-call events and plain-text deltas into a
//! `StreamedResponse`. Callers still get the legacy text fallback when the
//! provider does not emit structured tool-call events.

use anyhow::Result;
use futures::StreamExt;
use runie_core::event::{AgentEvent, Event};
use runie_core::llm_event::LLMEvent;
use runie_core::message::ChatMessage;
use runie_core::provider::Provider;
use runie_core::tool_markers::strip_tool_markers;
use runie_core::tool_parser::{parse_tool_calls_fallible, ParsedToolCall, ToolParseError};
use serde_json::Value;
use std::collections::HashMap;
use std::ops::ControlFlow;
use std::sync::{Arc, Mutex};

use crate::think_filter::ThinkFilter;

/// Emit type: Arc<Mutex<dyn FnMut(Event) + Send + Sync>>
pub type EmitFn = Arc<Mutex<dyn FnMut(Event) + Send + Sync>>;

/// A fully streamed assistant response.
#[derive(Debug, Clone)]
pub struct StreamedResponse {
    pub text: String,
    pub tool_calls: Vec<ParsedToolCall>,
    pub parse_errors: Vec<ToolParseError>,
    pub reasoning: Option<String>,
}

#[derive(Debug, Default)]
struct ToolCallAccumulator {
    name: String,
    arguments: String,
}

struct StreamState {
    text: String,
    reasoning: Option<String>,
    accumulators: HashMap<String, ToolCallAccumulator>,
    tool_calls: Vec<ParsedToolCall>,
    command_id: String,
    emit: EmitFn,
    think_filter: ThinkFilter,
}

impl StreamState {
    fn new(command_id: &str, emit: EmitFn) -> Self {
        Self {
            text: String::new(),
            reasoning: None,
            accumulators: HashMap::new(),
            tool_calls: Vec::new(),
            command_id: command_id.to_string(),
            emit,
            think_filter: ThinkFilter::new(),
        }
    }

    fn handle_flush(&mut self) -> Result<()> {
        let flushed = self.think_filter.flush();
        for ev in flushed {
            if let ControlFlow::Break(r) = self.handle_event(ev) {
                r?;
            }
        }
        Ok(())
    }

    fn handle_event(&mut self, event: LLMEvent) -> ControlFlow<Result<()>> {
        match event {
            LLMEvent::TextDelta(delta) => self.on_text_delta(delta),
            LLMEvent::ThinkingDelta(delta) => self.on_thinking_delta(delta),
            LLMEvent::ToolCallStart { id, name } => self.on_tool_start(id, name),
            LLMEvent::ToolCallInputDelta { id, delta } => self.on_tool_input(id, delta),
            LLMEvent::ToolCallEnd { id } => self.on_tool_end(id),
            LLMEvent::Finish { .. } => ControlFlow::Break(Ok(())),
            LLMEvent::Error(e) => ControlFlow::Break(Err(anyhow::anyhow!("LLM error: {:?}", e))),
            _ => ControlFlow::Continue(()),
        }
    }

    fn on_text_delta(&mut self, delta: String) -> ControlFlow<Result<()>> {
        self.text.push_str(&delta);
        emit_now(
            &self.emit,
            AgentEvent::ResponseDelta {
                id: self.command_id.clone(),
                content: delta,
            },
        );
        ControlFlow::Continue(())
    }

    fn on_thinking_delta(&mut self, delta: String) -> ControlFlow<Result<()>> {
        self.reasoning
            .get_or_insert_with(String::new)
            .push_str(&delta);
        ControlFlow::Continue(())
    }

    fn on_tool_start(&mut self, id: String, name: String) -> ControlFlow<Result<()>> {
        self.accumulators.entry(id).or_default().name = name;
        ControlFlow::Continue(())
    }

    fn on_tool_input(&mut self, id: String, delta: String) -> ControlFlow<Result<()>> {
        self.accumulators
            .entry(id)
            .or_default()
            .arguments
            .push_str(&delta);
        ControlFlow::Continue(())
    }

    fn on_tool_end(&mut self, id: String) -> ControlFlow<Result<()>> {
        if let Some(acc) = self.accumulators.remove(&id) {
            if let Some(call) = finish_tool_call(id, acc) {
                self.tool_calls.push(call);
            }
        }
        ControlFlow::Continue(())
    }

    fn finish_remaining_tools(&mut self) {
        let remaining: Vec<(String, ToolCallAccumulator)> =
            self.accumulators.drain().collect();
        for (id, acc) in remaining {
            if let Some(call) = finish_tool_call(id, acc) {
                self.tool_calls.push(call);
            }
        }
    }

    fn into_response(mut self) -> StreamedResponse {
        self.finish_remaining_tools();
        let mut parse_errors = Vec::new();
        if self.tool_calls.is_empty() && !self.text.is_empty() {
            for result in parse_tool_calls_fallible(&self.text) {
                match result {
                    Ok(call) => self.tool_calls.push(call),
                    Err(err) => parse_errors.push(err),
                }
            }
        }
        self.text = strip_tool_markers(&self.text);
        StreamedResponse {
            text: self.text,
            tool_calls: self.tool_calls,
            parse_errors,
            reasoning: self.reasoning,
        }
    }
}

/// Stream the provider response, accumulating text, reasoning, and tool calls.
pub async fn stream_response(
    provider: &dyn Provider,
    command_id: &str,
    messages: &[ChatMessage],
    tools: Vec<Value>,
    emit: EmitFn,
) -> Result<StreamedResponse> {
    let mut state = StreamState::new(command_id, emit);
    let mut stream = provider.generate_with_tools(messages.to_vec(), tools);

    while let Some(raw) = stream.next().await {
        let raw = raw?;
        let is_finish = matches!(&raw, LLMEvent::Finish { .. });
        let events = state.think_filter.feed(raw);
        if events.is_empty() && is_finish {
            state.handle_flush()?;
            return Ok(state.into_response());
        }
        for ev in events {
            if let ControlFlow::Break(result) = state.handle_event(ev) {
                return result.map(|_| Ok(state.into_response()))?;
            }
        }
    }

    Ok(state.into_response())
}

fn finish_tool_call(id: String, acc: ToolCallAccumulator) -> Option<ParsedToolCall> {
    if acc.name.is_empty() {
        return None;
    }
    let args: Value = serde_json::from_str(&acc.arguments).unwrap_or(Value::Null);
    Some(ParsedToolCall {
        name: acc.name,
        args,
        id: Some(id),
    })
}

fn emit_now(emit: &EmitFn, event: Event) {
    let mut emit = emit.lock().unwrap_or_else(|p| p.into_inner());
    emit(event);
}

#[cfg(test)]
mod tests {
    use super::*;
    use runie_core::llm_event::StopReason;

    struct TestProvider {
        events: Vec<LLMEvent>,
    }

    impl Provider for TestProvider {
        fn generate(
            &self,
            _messages: Vec<ChatMessage>,
        ) -> std::pin::Pin<Box<dyn futures::Stream<Item = Result<LLMEvent>> + Send + '_>> {
            let events = self.events.clone();
            Box::pin(futures::stream::iter(events.into_iter().map(Ok)))
        }
    }

    #[tokio::test]
    async fn accumulates_text_and_tool_calls() {
        let provider = TestProvider {
            events: vec![
                LLMEvent::TextDelta("I'll ".into()),
                LLMEvent::TextDelta("read.".into()),
                LLMEvent::ToolCallStart {
                    id: "call_1".into(),
                    name: "read_file".into(),
                },
                LLMEvent::ToolCallInputDelta {
                    id: "call_1".into(),
                    delta: "{\"path\":\"Cargo.toml\"}".into(),
                },
                LLMEvent::ToolCallEnd { id: "call_1".into() },
                LLMEvent::Finish {
                    reason: StopReason::ToolCalls,
                },
            ],
        };
        let emit: EmitFn = Arc::new(Mutex::new(|_| ()));
        let result = stream_response(&provider, "cmd", &[], vec![], emit)
            .await
            .unwrap();

        assert_eq!(result.text, "I'll read.");
        assert_eq!(result.tool_calls.len(), 1);
        assert_eq!(result.tool_calls[0].name, "read_file");
        assert_eq!(result.tool_calls[0].args["path"], "Cargo.toml");
        assert_eq!(result.tool_calls[0].id, Some("call_1".into()));
    }

    #[tokio::test]
    async fn falls_back_to_text_parsing_when_no_tool_events() {
        let provider = TestProvider {
            events: vec![
                LLMEvent::TextDelta(r#"{"name":"bash","arguments":{"command":"ls"}}"#.into()),
                LLMEvent::Finish {
                    reason: StopReason::Stop,
                },
            ],
        };
        let emit: EmitFn = Arc::new(Mutex::new(|_| ()));
        let result = stream_response(&provider, "cmd", &[], vec![], emit)
            .await
            .unwrap();

        assert_eq!(result.tool_calls.len(), 1);
        assert_eq!(result.tool_calls[0].name, "bash");
        assert_eq!(result.tool_calls[0].args["command"], "ls");
        assert!(result.text.is_empty());
    }

    #[tokio::test]
    async fn strips_tool_artifacts_from_text() {
        let provider = TestProvider {
            events: vec![
                LLMEvent::TextDelta(
                    "→ ```json{\"name\":\"list_dir\",\"arguments\":{\"path\":\".\"}}"
                        .into(),
                ),
                LLMEvent::TextDelta("Here's the current directory.".into()),
                LLMEvent::Finish {
                    reason: StopReason::Stop,
                },
            ],
        };
        let emit: EmitFn = Arc::new(Mutex::new(|_| ()));
        let result = stream_response(&provider, "cmd", &[], vec![], emit)
            .await
            .unwrap();

        assert_eq!(result.tool_calls.len(), 1);
        assert_eq!(result.tool_calls[0].name, "list_dir");
        assert_eq!(result.text, "Here's the current directory.");
    }

    // ========================================================================
    // Layer 2 — ThinkFilter integration tests
    // ========================================================================

    /// Layer 2: ThinkFilter extracts inline <tool_call> tags as thinking.
    #[tokio::test]
    async fn think_filter_extracts_inline_tool_call() {
        let provider = TestProvider {
            events: vec![
                LLMEvent::TextDelta("Let me ".into()),
                LLMEvent::TextDelta("<tool_call>analyzing".into()),
                LLMEvent::TextDelta("</tool_call>done".into()),
                LLMEvent::Finish {
                    reason: StopReason::Stop,
                },
            ],
        };
        let emit: EmitFn = Arc::new(Mutex::new(|_| ()));
        let result = stream_response(&provider, "cmd", &[], vec![], emit)
            .await
            .unwrap();

        // "Let me " + "done" should be text, inline thinking stripped
        assert!(result.text.contains("Let me"));
        assert!(result.text.contains("done"));
        assert!(!result.text.contains("analyzing"));
    }

    /// Layer 2: ThinkFilter handles partial tag at chunk boundary.
    #[tokio::test]
    async fn think_filter_partial_tag_boundary() {
        let provider = TestProvider {
            events: vec![
                LLMEvent::TextDelta("<tool_call>think".into()),
                LLMEvent::TextDelta("ing</tool_call>text".into()),
                LLMEvent::Finish {
                    reason: StopReason::Stop,
                },
            ],
        };
        let emit: EmitFn = Arc::new(Mutex::new(|_| ()));
        let result = stream_response(&provider, "cmd", &[], vec![], emit)
            .await
            .unwrap();

        assert!(result.text.contains("text"));
        assert!(!result.text.contains("thinking"));
    }

    /// Layer 2: ThinkFilter passthrough for structured ThinkingDelta.
    #[tokio::test]
    async fn think_filter_passthrough_thinking_delta() {
        let provider = TestProvider {
            events: vec![
                LLMEvent::ThinkingStart { id: "test".into() },
                LLMEvent::ThinkingDelta("reasoning".into()),
                LLMEvent::ThinkingEnd { id: "test".into() },
                LLMEvent::Finish {
                    reason: StopReason::Stop,
                },
            ],
        };
        let emit: EmitFn = Arc::new(Mutex::new(|_| ()));
        let result = stream_response(&provider, "cmd", &[], vec![], emit)
            .await
            .unwrap();

        // Structured thinking should not appear in text output
        assert!(!result.text.contains("reasoning"));
    }

    /// Layer 2: ThinkFilter flush at stream end handles unclosed block.
    #[tokio::test]
    async fn think_filter_flush_unclosed_block() {
        let provider = TestProvider {
            events: vec![
                LLMEvent::TextDelta("<thinking>unclosed".into()),
                LLMEvent::Finish {
                    reason: StopReason::Stop,
                },
            ],
        };
        let emit: EmitFn = Arc::new(Mutex::new(|_| ()));
        let result = stream_response(&provider, "cmd", &[], vec![], emit)
            .await
            .unwrap();

        // Should complete without error; thinking stripped
        assert!(!result.text.contains("unclosed"));
    }

    /// Layer 2: ThinkFilter no regression for plain text without tags.
    #[tokio::test]
    async fn think_filter_no_regression_plain_text() {
        let provider = TestProvider {
            events: vec![
                LLMEvent::TextDelta("Hello ".into()),
                LLMEvent::TextDelta("world!".into()),
                LLMEvent::Finish {
                    reason: StopReason::Stop,
                },
            ],
        };
        let emit: EmitFn = Arc::new(Mutex::new(|_| ()));
        let result = stream_response(&provider, "cmd", &[], vec![], emit)
            .await
            .unwrap();

        assert_eq!(result.text, "Hello world!");
    }

    /// Layer 2: ThinkFilter handles nested <tool_call> tags.
    #[tokio::test]
    async fn think_filter_nested_tool_call_tags() {
        let provider = TestProvider {
            events: vec![
                LLMEvent::TextDelta("<tool_call>first</tool_call><tool_call>second</tool_call>rest"
                    .into()),
                LLMEvent::Finish {
                    reason: StopReason::Stop,
                },
            ],
        };
        let emit: EmitFn = Arc::new(Mutex::new(|_| ()));
        let result = stream_response(&provider, "cmd", &[], vec![], emit)
            .await
            .unwrap();

        assert!(result.text.contains("rest"));
        assert!(!result.text.contains("first"));
        assert!(!result.text.contains("second"));
    }

    /// Layer 2: ThinkFilter with <thinking> tag variant.
    #[tokio::test]
    async fn think_filter_thinking_tag_variant() {
        let provider = TestProvider {
            events: vec![
                LLMEvent::TextDelta("<thinking>reasoning</thinking>answer".into()),
                LLMEvent::Finish {
                    reason: StopReason::Stop,
                },
            ],
        };
        let emit: EmitFn = Arc::new(Mutex::new(|_| ()));
        let result = stream_response(&provider, "cmd", &[], vec![], emit)
            .await
            .unwrap();

        assert!(result.text.contains("answer"));
        assert!(!result.text.contains("reasoning"));
    }
}
