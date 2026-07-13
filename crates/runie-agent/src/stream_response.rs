//! Stream response handling for the agent loop.
//!
//! Normalizes provider-native tool-call events and plain-text deltas into a
//! `StreamedResponse`. Callers still get the legacy text fallback when the
//! provider does not emit structured tool-call events.
//!
//! The core streaming logic is shared with the headless runner via
//! `streaming_parser::SharedStreamState`.

use anyhow::Result;
use futures::StreamExt;
use runie_core::event::Event;
use runie_core::message::ChatMessage;
use runie_core::provider::Provider;
use runie_core::provider_event::ProviderEvent;
use runie_core::tool::{ParsedToolCall, ToolParseError};
use serde_json::Value;
use std::ops::ControlFlow;
use std::sync::Arc;

use crate::streaming_parser::{SharedResponse, SharedStreamState};
use crate::think_filter::ThinkFilter;

/// Emit function type: a synchronous callable that ships an event.
///
/// Replaces the previous `Arc<Mutex<dyn FnMut>>` which locked per-token.
/// Clones are cheap (Arc-wrapped).
pub type EmitFn = Arc<dyn Fn(Event) + Send + Sync>;

/// A fully streamed assistant response.
#[derive(Debug, Clone)]
pub struct StreamedResponse {
    pub text: String,
    pub tool_calls: Vec<ParsedToolCall>,
    pub parse_errors: Vec<ToolParseError>,
    pub reasoning: Option<String>,
}

struct StreamState {
    shared: SharedStreamState,
    reasoning: Option<String>,
    command_id: String,
    emit: EmitFn,
    think_filter: ThinkFilter,
}

impl StreamState {
    fn new(command_id: &str, emit: EmitFn) -> Self {
        Self {
            shared: SharedStreamState::new(),
            reasoning: None,
            command_id: command_id.to_owned(),
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

    fn handle_event(&mut self, event: ProviderEvent) -> ControlFlow<Result<()>> {
        match event {
            ProviderEvent::TextDelta(delta) => self.on_text_delta(delta),
            ProviderEvent::ThinkingStart { .. } => {
                // Forward to the TUI: runie-core opens a Reasoning part on the
                // assistant message, which later becomes the expandable
                // thought post. Without these events the thought renders
                // duration-only and the reasoning is lost.
                (self.emit)(runie_core::Event::ThinkingStart {
                    id: self.command_id.clone(),
                });
                ControlFlow::Continue(())
            }
            ProviderEvent::ThinkingDelta(delta) => self.on_thinking_delta(delta),
            ProviderEvent::ThinkingEnd { .. } => {
                (self.emit)(runie_core::Event::ThinkingEnd {
                    id: self.command_id.clone(),
                });
                ControlFlow::Continue(())
            }
            ProviderEvent::ToolCallStart { id, name } => self.on_tool_start(id, name),
            ProviderEvent::ToolCallInputDelta { id, delta } => self.on_tool_input(id, delta),
            ProviderEvent::ToolCallEnd { id } => self.on_tool_end(id),
            ProviderEvent::Finish { .. } => ControlFlow::Break(Ok(())),
            ProviderEvent::Error(e) => {
                ControlFlow::Break(Err(anyhow::anyhow!("Model error: {:?}", e)))
            }
            _ => ControlFlow::Continue(()),
        }
    }

    fn on_text_delta(&mut self, delta: String) -> ControlFlow<Result<()>> {
        // Accumulate text in shared state; emit ResponseDelta directly.
        self.shared.push_text(&delta);

        (self.emit)(runie_core::Event::ResponseDelta {
            id: self.command_id.clone(),
            content: delta,
        });
        ControlFlow::Continue(())
    }

    fn on_thinking_delta(&mut self, delta: String) -> ControlFlow<Result<()>> {
        self.reasoning
            .get_or_insert_with(String::new)
            .push_str(&delta);
        (self.emit)(runie_core::Event::ThinkingDelta {
            id: self.command_id.clone(),
            content: delta,
        });
        ControlFlow::Continue(())
    }

    fn on_tool_start(&mut self, id: String, name: String) -> ControlFlow<Result<()>> {
        // Accumulate tool start in shared state; no separate emit needed for TUI.
        self.shared.start_tool(&id, &name);
        ControlFlow::Continue(())
    }

    fn on_tool_input(&mut self, id: String, delta: String) -> ControlFlow<Result<()>> {
        self.shared.append_tool_input(&id, &delta);
        ControlFlow::Continue(())
    }

    fn on_tool_end(&mut self, id: String) -> ControlFlow<Result<()>> {
        self.shared.finish_tool(&id);
        ControlFlow::Continue(())
    }

    fn into_response(self) -> StreamedResponse {
        let SharedResponse {
            text,
            tool_calls,
            parse_errors,
        } = self.shared.into_response();
        StreamedResponse {
            text,
            tool_calls,
            parse_errors,
            reasoning: self.reasoning,
        }
    }
}

/// Stream the provider response, accumulating text, reasoning, and tool calls.
///
/// The `cancel_token` is checked **before** yielding each event. When cancelled
/// (e.g. via `/new` → `AbortTurn`), the loop exits immediately so no further
/// `ResponseDelta` events are emitted after the abort signal.
pub async fn stream_response(
    provider: &dyn Provider,
    command_id: &str,
    messages: &[ChatMessage],
    tools: Vec<Value>,
    emit: EmitFn,
    cancel_token: tokio_util::sync::CancellationToken,
) -> Result<StreamedResponse> {
    let mut state = StreamState::new(command_id, emit);
    let mut stream = provider.generate_with_tools(messages.to_vec(), tools);

    loop {
        tokio::select! {
            biased;

            // Bail immediately when cancelled — prevents stale events after `/new`.
            _ = cancel_token.cancelled() => {
                break;
            }

            raw = stream.next() => {
                match raw {
                    Some(Ok(raw)) => {
                        let is_finish = matches!(&raw, ProviderEvent::Finish { .. });
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
                    Some(Err(e)) => {
                        return Err(e);
                    }
                    None => {
                        // Stream ended cleanly.
                        return Ok(state.into_response());
                    }
                }
            }
        }
    }

    // Cancelled — return what we've accumulated (discarded by callers on abort).
    Ok(state.into_response())
}

#[cfg(test)]
mod tests {
    use super::*;
    use runie_core::provider_event::StopReason;

    struct TestProvider {
        events: Vec<ProviderEvent>,
    }

    impl Provider for TestProvider {
        fn generate(
            &self,
            _messages: Vec<ChatMessage>,
        ) -> std::pin::Pin<Box<dyn futures::Stream<Item = Result<ProviderEvent>> + Send + '_>>
        {
            let events = self.events.clone();
            Box::pin(futures::stream::iter(events.into_iter().map(Ok)))
        }
    }

    #[tokio::test]
    async fn accumulates_text_and_tool_calls() {
        let provider = TestProvider {
            events: vec![
                ProviderEvent::TextDelta("I'll ".into()),
                ProviderEvent::TextDelta("read.".into()),
                ProviderEvent::ToolCallStart {
                    id: "call_1".into(),
                    name: "read_file".into(),
                },
                ProviderEvent::ToolCallInputDelta {
                    id: "call_1".into(),
                    delta: "{\"path\":\"Cargo.toml\"}".into(),
                },
                ProviderEvent::ToolCallEnd {
                    id: "call_1".into(),
                },
                ProviderEvent::Finish {
                    reason: StopReason::ToolCalls,
                },
            ],
        };
        let emit: EmitFn = Arc::new(|_| ());
        let result = stream_response(
            &provider,
            "cmd",
            &[],
            vec![],
            emit,
            tokio_util::sync::CancellationToken::new(),
        )
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
                ProviderEvent::TextDelta(r#"{"name":"bash","arguments":{"command":"ls"}}"#.into()),
                ProviderEvent::Finish {
                    reason: StopReason::Stop,
                },
            ],
        };
        let emit: EmitFn = Arc::new(|_| ());
        let result = stream_response(
            &provider,
            "cmd",
            &[],
            vec![],
            emit,
            tokio_util::sync::CancellationToken::new(),
        )
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
                ProviderEvent::TextDelta(
                    "→ ```json{\"name\":\"list_dir\",\"arguments\":{\"path\":\".\"}}".into(),
                ),
                ProviderEvent::TextDelta("Here's the current directory.".into()),
                ProviderEvent::Finish {
                    reason: StopReason::Stop,
                },
            ],
        };
        let emit: EmitFn = Arc::new(|_| ());
        let result = stream_response(
            &provider,
            "cmd",
            &[],
            vec![],
            emit,
            tokio_util::sync::CancellationToken::new(),
        )
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
                ProviderEvent::TextDelta("Let me ".into()),
                ProviderEvent::TextDelta("<tool_call>analyzing".into()),
                ProviderEvent::TextDelta("</tool_call>done".into()),
                ProviderEvent::Finish {
                    reason: StopReason::Stop,
                },
            ],
        };
        let emit: EmitFn = Arc::new(|_| ());
        let result = stream_response(
            &provider,
            "cmd",
            &[],
            vec![],
            emit,
            tokio_util::sync::CancellationToken::new(),
        )
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
                ProviderEvent::TextDelta("<tool_call>think".into()),
                ProviderEvent::TextDelta("ing</tool_call>text".into()),
                ProviderEvent::Finish {
                    reason: StopReason::Stop,
                },
            ],
        };
        let emit: EmitFn = Arc::new(|_| ());
        let result = stream_response(
            &provider,
            "cmd",
            &[],
            vec![],
            emit,
            tokio_util::sync::CancellationToken::new(),
        )
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
                ProviderEvent::ThinkingStart { id: "test".into() },
                ProviderEvent::ThinkingDelta("reasoning".into()),
                ProviderEvent::ThinkingEnd { id: "test".into() },
                ProviderEvent::Finish {
                    reason: StopReason::Stop,
                },
            ],
        };
        let emit: EmitFn = Arc::new(|_| ());
        let result = stream_response(
            &provider,
            "cmd",
            &[],
            vec![],
            emit,
            tokio_util::sync::CancellationToken::new(),
        )
        .await
        .unwrap();

        // Structured thinking should not appear in text output
        assert!(!result.text.contains("reasoning"));
    }

    /// Structured thinking must reach the TUI: runie-core turns
    /// ThinkingStart/Delta/End events into the expandable thought post.
    /// Swallowing them here (previously the case) left the thought
    /// duration-only — a dead `[+]` affordance and lost reasoning.
    #[tokio::test]
    async fn thinking_events_are_forwarded_to_the_tui() {
        let provider = TestProvider {
            events: vec![
                ProviderEvent::ThinkingStart { id: "reasoning".into() },
                ProviderEvent::ThinkingDelta("Let me think".into()),
                ProviderEvent::ThinkingDelta(" about this.".into()),
                ProviderEvent::ThinkingEnd { id: "reasoning".into() },
                ProviderEvent::TextDelta("The answer".into()),
                ProviderEvent::Finish {
                    reason: StopReason::Stop,
                },
            ],
        };
        let emitted: Arc<std::sync::Mutex<Vec<runie_core::Event>>> =
            Arc::new(std::sync::Mutex::new(Vec::new()));
        let cap = emitted.clone();
        let emit: EmitFn = Arc::new(move |e| cap.lock().unwrap().push(e));
        let result = stream_response(
            &provider,
            "cmd",
            &[],
            vec![],
            emit,
            tokio_util::sync::CancellationToken::new(),
        )
        .await
        .unwrap();

        let events = emitted.lock().unwrap();
        assert!(
            events
                .iter()
                .any(|e| matches!(e, runie_core::Event::ThinkingStart { .. })),
            "ThinkingStart must be forwarded: {events:?}"
        );
        let thinking: String = events
            .iter()
            .filter_map(|e| match e {
                runie_core::Event::ThinkingDelta { content, .. } => Some(content.clone()),
                _ => None,
            })
            .collect();
        assert_eq!(thinking, "Let me think about this.");
        assert!(
            events
                .iter()
                .any(|e| matches!(e, runie_core::Event::ThinkingEnd { .. })),
            "ThinkingEnd must be forwarded: {events:?}"
        );
        // Reasoning still accumulates for the message history.
        assert_eq!(result.reasoning.as_deref(), Some("Let me think about this."));
    }

    /// Layer 2: ThinkFilter flush at stream end handles unclosed block.
    #[tokio::test]
    async fn think_filter_flush_unclosed_block() {
        let provider = TestProvider {
            events: vec![
                ProviderEvent::TextDelta("<thinking>unclosed".into()),
                ProviderEvent::Finish {
                    reason: StopReason::Stop,
                },
            ],
        };
        let emit: EmitFn = Arc::new(|_| ());
        let result = stream_response(
            &provider,
            "cmd",
            &[],
            vec![],
            emit,
            tokio_util::sync::CancellationToken::new(),
        )
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
                ProviderEvent::TextDelta("Hello ".into()),
                ProviderEvent::TextDelta("world!".into()),
                ProviderEvent::Finish {
                    reason: StopReason::Stop,
                },
            ],
        };
        let emit: EmitFn = Arc::new(|_| ());
        let result = stream_response(
            &provider,
            "cmd",
            &[],
            vec![],
            emit,
            tokio_util::sync::CancellationToken::new(),
        )
        .await
        .unwrap();

        assert_eq!(result.text, "Hello world!");
    }

    /// Layer 2: ThinkFilter handles nested <tool_call> tags.
    #[tokio::test]
    async fn think_filter_nested_tool_call_tags() {
        let provider = TestProvider {
            events: vec![
                ProviderEvent::TextDelta(
                    "<tool_call>first</tool_call><tool_call>second</tool_call>rest".into(),
                ),
                ProviderEvent::Finish {
                    reason: StopReason::Stop,
                },
            ],
        };
        let emit: EmitFn = Arc::new(|_| ());
        let result = stream_response(
            &provider,
            "cmd",
            &[],
            vec![],
            emit,
            tokio_util::sync::CancellationToken::new(),
        )
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
                ProviderEvent::TextDelta("<thinking>reasoning</thinking>answer".into()),
                ProviderEvent::Finish {
                    reason: StopReason::Stop,
                },
            ],
        };
        let emit: EmitFn = Arc::new(|_| ());
        let result = stream_response(
            &provider,
            "cmd",
            &[],
            vec![],
            emit,
            tokio_util::sync::CancellationToken::new(),
        )
        .await
        .unwrap();

        assert!(result.text.contains("answer"));
        assert!(!result.text.contains("reasoning"));
    }

    // ========================================================================
    // Layer 1 — Stream error propagation
    // ========================================================================

    /// Layer 1: Provider stream error propagates as Err.
    #[tokio::test]
    async fn stream_error_propagates() {
        struct ErrorProvider;
        impl Provider for ErrorProvider {
            fn generate(
                &self,
                _: Vec<ChatMessage>,
            ) -> std::pin::Pin<Box<dyn futures::Stream<Item = Result<ProviderEvent>> + Send + '_>>
            {
                Box::pin(futures::stream::iter([Err(anyhow::anyhow!(
                    "provider error"
                ))]))
            }
        }
        let emit: EmitFn = Arc::new(|_| ());
        let result = stream_response(
            &ErrorProvider,
            "cmd",
            &[],
            vec![],
            emit,
            tokio_util::sync::CancellationToken::new(),
        )
        .await;
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().to_string(), "provider error");
    }

    // ========================================================================
    // ISSUE D — MiniMax content leak regression tests
    // ========================================================================
    //
    // MiniMax puts reasoning (`<think>`) and tool calls
    // (`<minimax:tool_call>` / `<tool_call>` / inline `{"name","arguments"}`)
    // INSIDE the SSE `delta.content`. The live feed must never render those raw
    // tags/JSON: thinking must travel via the ThinkingDelta path and tool calls
    // via structured tool events.

    /// Drive an SSE fixture through the full provider→agent streaming path and
    /// return every `Event` emitted to the live feed plus the final response.
    async fn drive_minimax_fixture(
        name: &str,
    ) -> (Vec<runie_core::Event>, StreamedResponse) {
        use runie_provider::openai::stream::replay_sse;
        let events =
            replay_sse(&runie_testing::fixtures::minimax::fixture(name));
        let provider = TestProvider { events };
        let captured: Arc<std::sync::Mutex<Vec<runie_core::Event>>> =
            Arc::new(std::sync::Mutex::new(Vec::new()));
        let cap = captured.clone();
        let emit: EmitFn = Arc::new(move |ev| cap.lock().unwrap().push(ev));
        let result = stream_response(
            &provider,
            "cmd",
            &[],
            vec![],
            emit,
            tokio_util::sync::CancellationToken::new(),
        )
        .await
        .unwrap();
        let emitted = std::mem::take(&mut *captured.lock().unwrap());
        (emitted, result)
    }

    fn joined_response_deltas(emitted: &[runie_core::Event]) -> String {
        emitted
            .iter()
            .filter_map(|e| match e {
                runie_core::Event::ResponseDelta { content, .. } => {
                    Some(content.as_str())
                }
                _ => None,
            })
            .collect::<Vec<_>>()
            .join("")
    }

    fn assert_no_live_leak(joined: &str) {
        for forbidden in [
            "<think",
            "</think>",
            "<minimax:tool_call",
            "</minimax:tool_call>",
            "</invoke>",
            "\"name\"",
            "\"arguments\"",
        ] {
            assert!(
                !joined.contains(forbidden),
                "live feed leaked {forbidden:?}; joined ResponseDelta = {joined:?}"
            );
        }
    }

    #[tokio::test]
    async fn minimax_m27_content_does_not_leak_tags_into_live_feed() {
        let (emitted, result) =
            drive_minimax_fixture("m27_multi_tool_readme.sse").await;
        let joined = joined_response_deltas(&emitted);
        assert_no_live_leak(&joined);

        // Reasoning is delivered via the thinking path, not as text.
        assert!(
            result
                .reasoning
                .as_deref()
                .unwrap_or("")
                .contains("read the README"),
            "thinking should reach the reasoning path; reasoning={:?}",
            result.reasoning
        );
        // Tool call is delivered as a structured tool call.
        assert!(
            result.tool_calls.iter().any(|tc| tc.name == "read_file"),
            "expected a structured read_file tool call; got {:?}",
            result.tool_calls
        );
    }

    #[tokio::test]
    async fn minimax_m3_inline_json_tool_call_does_not_leak_into_live_feed() {
        let (emitted, result) =
            drive_minimax_fixture("m3_list_files_call.sse").await;
        let joined = joined_response_deltas(&emitted);
        assert_no_live_leak(&joined);

        assert!(
            result.tool_calls.iter().any(|tc| tc.name == "list_dir"),
            "expected a structured list_dir tool call; got {:?}",
            result.tool_calls
        );
    }
}
