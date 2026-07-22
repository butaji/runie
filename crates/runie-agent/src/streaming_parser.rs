//! Shared streaming response parser for TUI and headless.
//
//! Both the TUI agent loop and the headless runner need to:
//! - Accumulate text deltas
//! - Accumulate tool calls via ToolStream
//! - Handle ProviderEvent variants
//! - Parse text fallback when no structured tool events arrive
//! - Strip tool markers from final text
//!
//! This module provides shared state and a single `stream_with_handler` that
//! both paths call with their own `StreamingHandler` implementation.

use anyhow::Result;
use runie_core::message::ChatMessage;
use runie_core::provider::Provider;
use runie_core::provider_event::ProviderEvent;
use runie_core::tool::{parse_tool_calls_fallible, ParsedToolCall, ToolParseError};
use runie_core::tool_markers::strip_tool_markers;
use runie_core::tool_stream::ToolStream;
use serde_json::Value;

/// Trait for handling events emitted by the streaming parser.
///
/// Each caller (TUI, headless) implements this to decide what to do with
/// text deltas, tool call lifecycle events, and errors.
pub trait StreamingHandler {
    /// Called for each text delta chunk.
    fn on_text_delta(&mut self, delta: String);
    /// Called when a tool call starts.
    fn on_tool_start(&mut self, id: String, name: String);
    /// Called for each tool call input delta.
    fn on_tool_input(&mut self, id: String, delta: String);
    /// Called when a tool call ends.
    fn on_tool_end(&mut self, id: String);
    /// Called when a stream finishes successfully.
    fn on_finish(&mut self);
    /// Called when a stream errors.
    fn on_error(&mut self, message: String) -> Result<()>;
    /// Return `true` to signal the stream should stop (e.g. cancellation).
    fn is_cancelled(&self) -> bool;
}

/// Shared streaming response state accumulated during a turn.
#[derive(Default)]
pub struct SharedStreamState {
    /// Accumulated raw text (before tool marker stripping).
    pub text: String,
    tool_stream: ToolStream,
    tool_calls: Vec<ParsedToolCall>,
}

impl SharedStreamState {
    pub fn new() -> Self {
        Self::default()
    }

    /// Accumulate a text delta into the shared text buffer.
    pub fn push_text(&mut self, delta: &str) {
        self.text.push_str(delta);
    }

    /// Push accumulated text and notify the handler.
    pub fn on_text_delta<H: StreamingHandler>(&mut self, handler: &mut H, delta: String) {
        self.text.push_str(&delta);
        handler.on_text_delta(delta);
    }

    /// Start a named tool call in the shared tool stream.
    pub fn start_tool(&mut self, id: &str, name: &str) {
        self.tool_stream.start(id, name);
    }

    /// Append input to an in-progress tool call.
    pub fn append_tool_input(&mut self, id: &str, delta: &str) {
        self.tool_stream.append(id, delta);
    }

    /// Finish a tool call and extract the completed `ParsedToolCall`.
    /// Does NOT call any handler callback — callers emit their own events.
    pub fn finish_tool(&mut self, id: &str) {
        if let Some(call) = self.tool_stream.finish(id) {
            self.tool_calls.push(call);
        }
    }

    /// Start a named tool call and notify the handler.
    pub fn on_tool_start<H: StreamingHandler>(&mut self, handler: &mut H, id: String, name: String) {
        self.start_tool(&id, &name);
        handler.on_tool_start(id, name);
    }

    /// Append input to an in-progress tool call and notify the handler.
    pub fn on_tool_input<H: StreamingHandler>(&mut self, handler: &mut H, id: String, delta: String) {
        self.append_tool_input(&id, &delta);
        handler.on_tool_input(id, delta);
    }

    /// End an in-progress tool call and notify the handler.
    /// Extracts the completed call from the shared tool stream.
    pub fn on_tool_end<H: StreamingHandler>(&mut self, handler: &mut H, id: String) {
        handler.on_tool_end(id.clone());
        if let Some(call) = self.tool_stream.finish(&id) {
            self.tool_calls.push(call);
        }
    }

    /// Finish any remaining open tool calls.
    pub fn finish_remaining_tools(&mut self) {
        let calls = self.tool_stream.finish_all();
        self.tool_calls.extend(calls);
    }

    /// Return the accumulated raw text (before marker stripping).
    pub fn raw_text(&self) -> &str {
        &self.text
    }

    /// Consume self and return the accumulated text, tool calls, and parse errors.
    /// When no structured tool events arrived, falls back to parsing the text.
    pub fn into_response(mut self) -> SharedResponse {
        self.finish_remaining_tools();
        let mut parse_errors = Vec::new();
        let mut tool_calls = Vec::new();
        std::mem::swap(&mut self.tool_calls, &mut tool_calls);

        if tool_calls.is_empty() && !self.text.is_empty() {
            for result in parse_tool_calls_fallible(&self.text) {
                match result {
                    Ok(call) => tool_calls.push(call),
                    Err(err) => parse_errors.push(err),
                }
            }
        }
        let text = strip_tool_markers(&self.text);
        SharedResponse { text, tool_calls, parse_errors }
    }
}

/// Result of parsing a provider stream.
#[derive(Debug)]
pub struct SharedResponse {
    pub text: String,
    pub tool_calls: Vec<ParsedToolCall>,
    pub parse_errors: Vec<ToolParseError>,
}

/// Stream a provider response, routing events through a `StreamingHandler`.
///
/// The `is_cancelled()` method is polled each iteration so that
/// cancellation (e.g. via `/new` → `AbortTurn`) exits immediately.
#[allow(clippy::too_many_lines)]
pub async fn stream_with_handler<H: StreamingHandler>(
    provider: &dyn Provider,
    messages: &[ChatMessage],
    tools: Vec<Value>,
    mut handler: H,
) -> Result<SharedResponse> {
    let mut state = SharedStreamState::new();
    let mut stream = provider.generate_with_tools(messages.to_vec(), tools);

    loop {
        // Check cancellation before each iteration — prevents stale events after `/new`.
        if handler.is_cancelled() {
            return Ok(state.into_response());
        }

        let raw = futures::StreamExt::next(&mut stream).await;
        match raw {
            Some(Ok(raw)) => match raw {
                ProviderEvent::TextDelta(delta) => {
                    state.on_text_delta(&mut handler, delta);
                }
                ProviderEvent::ToolCallStart { id, name } => {
                    state.on_tool_start(&mut handler, id, name);
                }
                ProviderEvent::ToolCallInputDelta { id, delta } => {
                    state.on_tool_input(&mut handler, id, delta);
                }
                ProviderEvent::ToolCallEnd { id } => {
                    state.on_tool_end(&mut handler, id);
                }
                ProviderEvent::Finish { .. } => {
                    handler.on_finish();
                    return Ok(state.into_response());
                }
                ProviderEvent::Error(e) => {
                    handler.on_error(format!("{:?}", e))?;
                    return Ok(state.into_response());
                }
                _ => {}
            },
            Some(Err(e)) => {
                return Err(e);
            }
            None => {
                return Ok(state.into_response());
            }
        }
    }
}
