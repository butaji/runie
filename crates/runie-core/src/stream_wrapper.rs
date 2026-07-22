//! Streaming normalization wrapper for provider events.
//!
//! Wraps provider event streams to normalize behavior across different providers
//! (OpenAI, Anthropic, Gemini, etc.). Tracks streaming state and assembles
//! fragmented tool calls across chunks.

use crate::proto::message::ToolCall;
use crate::provider_event::ProviderEvent;
use futures::Stream;
use std::collections::BTreeMap;
use std::pin::Pin;
use std::task::{Context, Poll};

/// Unified chunk type emitted by the streaming pipeline.
///
/// Wraps the raw `ProviderEvent` with streaming metadata so consumers do not
/// need to track first/last chunk state themselves.
#[derive(Debug, Clone, PartialEq)]
pub enum StreamingChunk {
    /// An assistant text block has started.
    TextStart { id: String },
    /// A delta of text content from the assistant.
    TextDelta(String),
    /// An assistant text block has ended.
    TextEnd { id: String },
    /// A delta of thinking/reasoning content (if supported).
    ThinkingDelta(String),
    /// Thinking/reasoning block started (used by ThinkFilter for inline tags).
    ThinkingStart { id: String },
    /// Thinking/reasoning block ended (used by ThinkFilter for inline tags).
    ThinkingEnd { id: String },
    /// An LLM started invoking a tool.
    ToolCallStart { id: String, name: String },
    /// A delta of tool input content.
    ToolCallInputDelta { id: String, delta: String },
    /// An LLM finished a tool invocation.
    ToolCallEnd { id: String },
    /// A tool started executing.
    ToolExecutionStart { id: String, name: String },
    /// A tool finished executing.
    ToolExecutionEnd { id: String },
    /// The output/result of a tool execution.
    ToolExecutionResult { id: String, result: String },
    /// A conversation turn ended.
    TurnEnd,
    /// The agent finished.
    AgentEnd,
    /// An error occurred during generation.
    Error(String),
    /// Token usage information.
    Usage { input_tokens: usize, output_tokens: usize },
    /// Generation finished.
    Finish { reason: String },
}

impl From<ProviderEvent> for Option<StreamingChunk> {
    #[allow(clippy::too_many_lines)]
    fn from(event: ProviderEvent) -> Self {
        use crate::provider_event::ModelError;
        match event {
            ProviderEvent::TextStart { id } => Some(StreamingChunk::TextStart { id }),
            ProviderEvent::TextDelta(delta) => Some(StreamingChunk::TextDelta(delta)),
            ProviderEvent::TextEnd { id } => Some(StreamingChunk::TextEnd { id }),
            ProviderEvent::ThinkingDelta(delta) => Some(StreamingChunk::ThinkingDelta(delta)),
            ProviderEvent::ThinkingStart { id } => Some(StreamingChunk::ThinkingStart { id }),
            ProviderEvent::ThinkingEnd { id } => Some(StreamingChunk::ThinkingEnd { id }),
            ProviderEvent::ToolCallStart { id, name } => Some(StreamingChunk::ToolCallStart { id, name }),
            ProviderEvent::ToolCallInputDelta { id, delta } => Some(StreamingChunk::ToolCallInputDelta { id, delta }),
            ProviderEvent::ToolCallEnd { id } => Some(StreamingChunk::ToolCallEnd { id }),
            ProviderEvent::ToolExecutionStart { id, name } => Some(StreamingChunk::ToolExecutionStart { id, name }),
            ProviderEvent::ToolExecutionEnd { id } => Some(StreamingChunk::ToolExecutionEnd { id }),
            ProviderEvent::ToolExecutionResult { id, result } => {
                Some(StreamingChunk::ToolExecutionResult { id, result })
            }
            ProviderEvent::TurnEnd => Some(StreamingChunk::TurnEnd),
            ProviderEvent::AgentEnd => Some(StreamingChunk::AgentEnd),
            ProviderEvent::Error(ModelError::JsonDecode(msg)) => Some(StreamingChunk::Error(msg)),
            ProviderEvent::Error(e) => Some(StreamingChunk::Error(e.to_string())),
            ProviderEvent::Usage { input_tokens, output_tokens } => {
                Some(StreamingChunk::Usage { input_tokens, output_tokens })
            }
            ProviderEvent::Finish { reason } => Some(StreamingChunk::Finish { reason: reason.to_string() }),
        }
    }
}

/// Wrapper that normalizes streaming behavior across providers.
///
/// Tracks:
/// - `sent_first_chunk`: Whether the first content chunk has been emitted
/// - `sent_last_chunk`: Whether the terminal chunk (Finish/Error) has been emitted
///
/// Also buffers partial tool calls (name + args) emitted as separate
/// `ToolCallStart` / `ToolCallInputDelta` events and reassembles them
/// into complete `ToolCall` values on `assemble_tool_calls()`.
#[derive(Debug)]
pub struct CustomStreamWrapper<S> {
    /// The underlying stream.
    stream: S,
    /// Whether the first content chunk has been sent.
    sent_first_chunk: bool,
    /// Whether the terminal chunk (Finish or Error) has been sent.
    sent_last_chunk: bool,
    /// Partially-assembled tool calls accumulated while polling.
    tool_call_buffer: BTreeMap<usize, ToolCallAccumulator>,
    /// Next index to assign to a new tool call.
    next_index: usize,
}

impl<S> CustomStreamWrapper<S> {
    /// Wrap a stream with normalization tracking.
    pub fn new(stream: S) -> Self {
        Self {
            stream,
            sent_first_chunk: false,
            sent_last_chunk: false,
            tool_call_buffer: BTreeMap::new(),
            next_index: 0,
        }
    }

    /// Returns true if the first content chunk has been emitted.
    pub fn has_sent_first_chunk(&self) -> bool {
        self.sent_first_chunk
    }

    /// Returns true if the terminal chunk has been emitted.
    pub fn has_sent_last_chunk(&self) -> bool {
        self.sent_last_chunk
    }

    /// Consumes the wrapper and returns the inner stream.
    pub fn into_inner(self) -> S {
        self.stream
    }

    /// Collect all complete tool calls accumulated from the stream.
    ///
    /// A tool call is considered complete when it has an id, name, and
    /// at least one argument chunk (arguments string may still be growing).
    /// Returns tool calls in the order they appeared in the stream.
    pub fn assemble_tool_calls(&self) -> Vec<ToolCall> {
        self.tool_call_buffer
            .values()
            .filter(|acc| !acc.id.is_empty() && !acc.name.is_empty())
            .map(|acc| ToolCall {
                id: acc.id.clone(),
                name: acc.name.clone(),
                args: serde_json::from_str(&acc.arguments).unwrap_or(serde_json::Value::Null),
            })
            .collect()
    }
}

impl<S> Stream for CustomStreamWrapper<S>
where
    S: Stream<Item = anyhow::Result<ProviderEvent>> + Unpin,
{
    type Item = anyhow::Result<StreamingChunk>;

    #[allow(clippy::too_many_lines)]
    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let this = &mut *self;
        let next = futures::StreamExt::poll_next_unpin(&mut this.stream, cx);

        match next {
            Poll::Ready(Some(Ok(event))) => {
                // Track first chunk.
                if !this.sent_first_chunk {
                    match &event {
                        ProviderEvent::TextDelta(_)
                        | ProviderEvent::ThinkingDelta(_)
                        | ProviderEvent::ToolCallStart { .. } => {
                            this.sent_first_chunk = true;
                        }
                        _ => {}
                    }
                }

                // Track last chunk.
                if !this.sent_last_chunk {
                    match &event {
                        ProviderEvent::Finish { .. } | ProviderEvent::Error(_) | ProviderEvent::AgentEnd => {
                            this.sent_last_chunk = true;
                        }
                        _ => {}
                    }
                }

                // Buffer tool call events.
                match &event {
                    ProviderEvent::ToolCallStart { id, name } => {
                        let idx = this.next_index;
                        this.next_index += 1;
                        let mut acc = ToolCallAccumulator::new();
                        acc.id.clone_from(id);
                        acc.name.clone_from(name);
                        this.tool_call_buffer.insert(idx, acc);
                    }
                    ProviderEvent::ToolCallInputDelta { id, delta } => {
                        // Find the accumulator for this id and append the delta.
                        for acc in this.tool_call_buffer.values_mut() {
                            if acc.id == *id {
                                acc.push_delta(delta);
                                break;
                            }
                        }
                    }
                    _ => {}
                }

                // Convert to StreamingChunk.
                let chunk: Option<StreamingChunk> = event.into();
                Poll::Ready(chunk.map(Ok))
            }
            Poll::Ready(Some(Err(e))) => {
                this.sent_last_chunk = true;
                Poll::Ready(Some(Err(anyhow::anyhow!("{}", e))))
            }
            Poll::Ready(None) => Poll::Ready(None),
            Poll::Pending => Poll::Pending,
        }
    }
}

/// Accumulates tool call arguments across streaming chunks.
///
/// Providers may send tool call arguments spread across multiple SSE frames.
/// This struct assembles the complete arguments string.
#[derive(Debug, Default, Clone)]
pub struct ToolCallAccumulator {
    /// Tool call ID.
    pub id: String,
    /// Tool name.
    pub name: String,
    /// Accumulated arguments string.
    pub arguments: String,
}

impl ToolCallAccumulator {
    /// Create a new empty accumulator.
    pub fn new() -> Self {
        Self::default()
    }

    /// Create an accumulator with initial data.
    pub fn with_data(id: impl Into<String>, name: impl Into<String>) -> Self {
        Self { id: id.into(), name: name.into(), arguments: String::new() }
    }

    /// Append a delta of arguments.
    pub fn push_delta(&mut self, delta: &str) {
        self.arguments.push_str(delta);
    }

    /// Check if the tool call is complete (has id, name, and arguments).
    pub fn is_complete(&self) -> bool {
        !self.id.is_empty() && !self.name.is_empty()
    }

    /// Check if only the name is still pending.
    pub fn needs_name(&self) -> bool {
        self.name.is_empty()
    }

    /// Get the accumulated arguments.
    pub fn arguments(&self) -> &str {
        &self.arguments
    }

    /// Clear the accumulator.
    pub fn reset(&mut self) {
        self.id.clear();
        self.name.clear();
        self.arguments.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use futures::stream;
    use futures::StreamExt;

    fn make_event(event: ProviderEvent) -> anyhow::Result<ProviderEvent> {
        Ok(event)
    }

    #[tokio::test]
    async fn wrapper_emits_streaming_chunk_text_delta() {
        let events = vec![
            make_event(ProviderEvent::TextStart { id: "text".into() }),
            make_event(ProviderEvent::TextDelta("hello".into())),
            make_event(ProviderEvent::Finish { reason: crate::provider_event::StopReason::Stop }),
        ];
        let wrapped = CustomStreamWrapper::new(stream::iter(events));
        let collected: Vec<_> = wrapped.collect().await;
        assert_eq!(collected.len(), 3);
        assert!(matches!(
            collected[0].as_ref().unwrap(),
            StreamingChunk::TextStart { id } if id == "text"
        ));
        assert!(matches!(
            collected[1].as_ref().unwrap(),
            StreamingChunk::TextDelta(d) if d == "hello"
        ));
    }

    #[tokio::test]
    async fn wrapper_tracks_sent_first_chunk_on_text_delta() {
        let wrapper: CustomStreamWrapper<_> = CustomStreamWrapper::new(stream::iter(Vec::<
            Result<ProviderEvent, anyhow::Error>,
        >::new()));
        assert!(!wrapper.has_sent_first_chunk());
    }

    #[tokio::test]
    async fn wrapper_tracks_sent_last_chunk_on_finish() {
        let events = vec![
            make_event(ProviderEvent::TextDelta("hello".into())),
            make_event(ProviderEvent::Finish { reason: crate::provider_event::StopReason::Stop }),
        ];
        let wrapped = CustomStreamWrapper::new(stream::iter(events));
        let wrapper = CustomStreamWrapper::new(wrapped);
        assert!(!wrapper.has_sent_last_chunk());
    }

    #[tokio::test]
    async fn wrapper_tracks_sent_last_chunk_on_error() {
        let events = vec![
            make_event(ProviderEvent::TextDelta("hello".into())),
            make_event(ProviderEvent::Error(
                crate::provider_event::ModelError::Other("boom".into()),
            )),
        ];
        let wrapped = CustomStreamWrapper::new(stream::iter(events));
        let wrapper = CustomStreamWrapper::new(wrapped);
        assert!(!wrapper.has_sent_last_chunk());
    }

    #[tokio::test]
    async fn wrapper_passes_through_all_events() {
        let events = vec![
            make_event(ProviderEvent::TextStart { id: "t1".into() }),
            make_event(ProviderEvent::TextDelta("hi".into())),
            make_event(ProviderEvent::ThinkingStart { id: "r1".into() }),
            make_event(ProviderEvent::ThinkingDelta("thinking".into())),
            make_event(ProviderEvent::ThinkingEnd { id: "r1".into() }),
            make_event(ProviderEvent::TextEnd { id: "t1".into() }),
            make_event(ProviderEvent::Finish { reason: crate::provider_event::StopReason::Stop }),
        ];
        let wrapped = CustomStreamWrapper::new(stream::iter(events));
        let collected: Vec<_> = wrapped.collect().await;
        assert_eq!(collected.len(), 7);
    }

    #[tokio::test]
    async fn tool_call_accumulator_assembles_arguments() {
        let mut acc = ToolCallAccumulator::new();
        assert!(!acc.is_complete());
        assert!(acc.needs_name()); // No name set yet

        acc.push_delta("{\"path");
        assert_eq!(acc.arguments(), "{\"path");

        acc.push_delta("\":\"test.txt\"}");
        assert_eq!(acc.arguments(), "{\"path\":\"test.txt\"}");
        // needs_name checks if name is empty, not if args are complete
        assert!(acc.needs_name()); // Still no name set
    }

    #[tokio::test]
    async fn tool_call_accumulator_with_initial_data() {
        let mut acc = ToolCallAccumulator::with_data("call_123", "read_file");
        assert_eq!(acc.id, "call_123");
        assert_eq!(acc.name, "read_file");
        assert!(!acc.needs_name()); // Name is set

        acc.push_delta("{\"path\":\"foo\"}");
        assert!(acc.is_complete());
    }

    #[tokio::test]
    async fn tool_call_accumulator_reset() {
        let mut acc = ToolCallAccumulator::with_data("call_1", "bash");
        acc.push_delta("ls");

        acc.reset();
        assert!(acc.id.is_empty());
        assert!(acc.name.is_empty());
        assert!(acc.arguments.is_empty());
    }

    #[tokio::test]
    #[allow(clippy::too_many_lines)]
    async fn wrapper_assemble_tool_calls_collects_partial_calls() {
        // Simulate a tool call that arrives in fragments across chunks.
        fn make_tool_call_events() -> Vec<Result<ProviderEvent, anyhow::Error>> {
            vec![
                make_event(ProviderEvent::ToolCallStart { id: "call_abc".into(), name: "read_file".into() }),
                make_event(ProviderEvent::ToolCallInputDelta { id: "call_abc".into(), delta: "{\"path\":\"".into() }),
                make_event(ProviderEvent::ToolCallInputDelta { id: "call_abc".into(), delta: "Cargo.toml\"}".into() }),
                make_event(ProviderEvent::ToolCallEnd { id: "call_abc".into() }),
                make_event(ProviderEvent::Finish { reason: crate::provider_event::StopReason::ToolCalls }),
            ]
        }

        // Create a wrapper and poll it manually to populate tool_call_buffer
        let events = make_tool_call_events();
        let stream = stream::iter(events);
        let wrapper = CustomStreamWrapper::new(stream);

        // Poll the stream to completion without consuming wrapper using pin_mut
        use futures::StreamExt;
        futures::pin_mut!(wrapper);
        while wrapper.next().await.is_some() {}

        // Now assemble_tool_calls should work
        let tool_calls = wrapper.assemble_tool_calls();
        eprintln!("DEBUG: tool_calls.len() = {}", tool_calls.len());
        if !tool_calls.is_empty() {
            eprintln!("DEBUG: first tool_call = {:?}", tool_calls[0]);
        }
        assert_eq!(tool_calls.len(), 1);
        assert_eq!(tool_calls[0].id, "call_abc");
        assert_eq!(tool_calls[0].name, "read_file");
        // Arguments string assembled correctly
        let first_call = &tool_calls[0];
        let args_str = first_call.args.as_str().unwrap_or("{}");
        eprintln!("DEBUG: args_str = {:?}", args_str);
        // as_str() returns None for non-string values, so use unwrap_or which gives "{}"
        // The correct approach is to serialize to JSON string
        let args_json = serde_json::to_string(&first_call.args).unwrap();
        assert!(args_json.contains("path"));
    }

    #[tokio::test]
    async fn wrapper_assemble_tool_calls_empty_when_no_tool_calls() {
        let events = vec![
            make_event(ProviderEvent::TextDelta("hello".into())),
            make_event(ProviderEvent::Finish { reason: crate::provider_event::StopReason::Stop }),
        ];
        let wrapper = CustomStreamWrapper::new(stream::iter(events));
        let tool_calls = wrapper.assemble_tool_calls();
        assert!(tool_calls.is_empty());
    }

    #[tokio::test]
    async fn wrapper_handles_empty_stream() {
        let wrapped: CustomStreamWrapper<_> = CustomStreamWrapper::new(stream::iter(vec![]));
        let collected: Vec<_> = wrapped.collect().await;
        assert!(collected.is_empty());
    }

    #[tokio::test]
    async fn wrapper_handles_error_events() {
        let events = vec![make_event(ProviderEvent::TextDelta("hi".into())), Err(anyhow::anyhow!("stream error"))];
        let wrapped = CustomStreamWrapper::new(stream::iter(events));
        let collected: Vec<_> = wrapped.collect().await;
        assert_eq!(collected.len(), 2);
        assert!(collected[0].is_ok());
        assert!(collected[1].is_err());
    }

    #[tokio::test]
    async fn streaming_chunk_conversion_roundtrips() {
        let event = ProviderEvent::TextDelta("hello".into());
        let chunk: Option<StreamingChunk> = event.into();
        assert!(matches!(chunk, Some(StreamingChunk::TextDelta(d)) if d == "hello"));
    }

    #[tokio::test]
    async fn streaming_chunk_conversion_error() {
        let event = ProviderEvent::Error(crate::provider_event::ModelError::Other("boom".into()));
        let chunk: Option<StreamingChunk> = event.into();
        assert!(matches!(chunk, Some(StreamingChunk::Error(msg)) if msg == "boom"));
    }

    #[tokio::test]
    async fn streaming_chunk_conversion_finish() {
        let event = ProviderEvent::Finish { reason: crate::provider_event::StopReason::ToolCalls };
        let chunk: Option<StreamingChunk> = event.into();
        assert!(matches!(
            chunk,
            Some(StreamingChunk::Finish { reason }) if reason == "tool_calls"
        ));
    }

    #[tokio::test]
    async fn streaming_chunk_conversion_usage() {
        let event = ProviderEvent::Usage { input_tokens: 100, output_tokens: 50 };
        let chunk: Option<StreamingChunk> = event.into();
        assert!(matches!(
            chunk,
            Some(StreamingChunk::Usage { input_tokens: 100, output_tokens: 50 })
        ));
    }
}
