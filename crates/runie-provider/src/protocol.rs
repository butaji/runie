//! Provider protocol trait for streaming LLM responses.
//!
//! This module defines the `ProviderProtocol` trait that abstracts over
//! provider-specific SSE frame parsing. New providers implement this trait
//! to get transport and framing for free.

use runie_core::provider_event::ProviderEvent;
use std::collections::HashMap;

/// Trait for provider-specific streaming protocol handling.
///
/// Implementors provide the state machine that transforms raw SSE frames
/// into `LLMEvent`s. Transport (HTTP), framing (SSE), and accumulation
/// are shared infrastructure.
///
/// # Type Parameters
/// - `Frame`: The provider-specific frame type parsed from SSE data.
/// - `State`: Accumulator state that persists across frames.
pub trait ProviderProtocol: Send + Sync {
    /// The frame type produced by this provider's SSE parser.
    type Frame: Send;

    /// The state accumulator for this provider.
    type State: Default + Send;

    /// Create initial state from the request.
    fn initial(&self, _request: &Request) -> Self::State {
        Self::State::default()
    }

    /// Process a single frame and return updated state plus emitted events.
    fn step(
        &self,
        state: Self::State,
        frame: Self::Frame,
    ) -> (Self::State, Vec<ProviderEvent>);

    /// Called when the stream halts (error or end). Flushes any pending state.
    fn on_halt(&self, state: Self::State) -> Vec<ProviderEvent> {
        let _ = state;
        Vec::new()
    }

    /// Returns true if this frame signals the end of the stream.
    fn terminal(&self, _frame: &Self::Frame) -> bool {
        false
    }
}

/// A minimal request context passed to protocol initializers.
#[derive(Debug, Clone)]
pub struct Request {
    pub model: String,
    pub tools: Vec<serde_json::Value>,
}

impl Request {
    pub fn new(model: impl Into<String>) -> Self {
        Self {
            model: model.into(),
            tools: Vec::new(),
        }
    }

    pub fn with_tools(mut self, tools: Vec<serde_json::Value>) -> Self {
        self.tools = tools;
        self
    }
}

/// A helper for tracking accumulated tool call state.
#[derive(Debug, Default, Clone)]
pub struct ToolAccumulator {
    pub id: String,
    pub name: String,
    pub arguments: String,
}

/// Manages multiple in-flight tool calls by index.
#[derive(Debug, Default)]
pub struct ToolRegistry<T = ToolAccumulator> {
    pub tools: HashMap<usize, T>,
    pub started: HashMap<String, ()>,
    pub ended: HashMap<String, ()>,
}

impl ToolRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn is_started(&self, id: &str) -> bool {
        self.started.contains_key(id)
    }

    pub fn mark_started(&mut self, id: String) {
        self.started.insert(id, ());
    }

    pub fn mark_ended(&mut self, id: String) {
        self.ended.insert(id, ());
    }

    pub fn is_ended(&self, id: &str) -> bool {
        self.ended.contains_key(id)
    }

    pub fn reset(&mut self) {
        *self = Self::new();
    }
}

impl ToolAccumulator {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn is_complete(&self) -> bool {
        !self.id.is_empty() && !self.name.is_empty()
    }
}

/// Marker trait for frame types that indicate end of stream.
pub trait TerminalFrame {
    fn is_terminal(&self) -> bool;
}

/// Wrapper for frames that implement terminal detection.
#[derive(Debug)]
pub struct TerminalWrapper<F> {
    pub inner: F,
    pub is_done: bool,
}

impl<F> TerminalWrapper<F> {
    pub fn new(inner: F) -> Self {
        Self { inner, is_done: false }
    }

    pub fn done(inner: F) -> Self {
        Self { inner, is_done: true }
    }
}

impl<F> TerminalFrame for TerminalWrapper<F> {
    fn is_terminal(&self) -> bool {
        self.is_done
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tool_registry_tracks_started_ended() {
        let mut reg = ToolRegistry::new();
        assert!(!reg.is_started("call_1"));
        reg.mark_started("call_1".to_string());
        assert!(reg.is_started("call_1"));
        assert!(!reg.is_ended("call_1"));
        reg.mark_ended("call_1".to_string());
        assert!(reg.is_ended("call_1"));
    }

    #[test]
    fn tool_accumulator_is_complete() {
        let mut acc = ToolAccumulator::new();
        assert!(!acc.is_complete());
        acc.id = "call_1".to_string();
        assert!(!acc.is_complete());
        acc.name = "read_file".to_string();
        assert!(acc.is_complete());
    }

    #[test]
    fn request_builder() {
        let req = Request::new("gpt-4o").with_tools(vec![serde_json::json!({"type": "function"})]);
        assert_eq!(req.model, "gpt-4o");
        assert_eq!(req.tools.len(), 1);
    }
}
