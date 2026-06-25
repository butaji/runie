//! Tool call streaming accumulator.
//!
//! Centralized state machine for buffering and assembling tool-call argument deltas
//! across streaming LLM responses. Replaces duplicate accumulators in
//! `runie-provider` (OpenAI protocol) and `runie-agent` (event-based).
//!
//! Usage:
//! ```ignore
//! let mut stream = ToolStream::new();
//! stream.start("call_1", "read_file");
//! stream.append("call_1", "{\"path\":");
//! stream.append("call_1", " \"README.md\"}");
//! let call = stream.finish("call_1"); // Some(ParsedToolCall { name: "read_file", ... })
//! ```

use serde_json::Value;

/// Accumulator for a single tool call's arguments.
#[derive(Debug, Default)]
pub struct Accumulator {
    pub name: String,
    pub arguments: String,
}

/// Tool call streaming state machine.
#[derive(Debug, Default)]
pub struct ToolStream {
    /// Active accumulators keyed by call id.
    pending: std::collections::HashMap<String, Accumulator>,
}

impl ToolStream {
    /// Create a new empty tool stream.
    pub fn new() -> Self {
        Self::default()
    }

    /// Start tracking a new tool call with the given id and name.
    pub fn start(&mut self, id: &str, name: &str) {
        self.pending.entry(id.to_string()).or_default().name = name.to_string();
    }

    /// Append argument delta to a tracked tool call.
    /// If the id is not being tracked, this is a no-op.
    pub fn append(&mut self, id: &str, delta: &str) {
        if let Some(acc) = self.pending.get_mut(id) {
            acc.arguments.push_str(delta);
        }
    }

    /// Finish and return a tool call, removing it from tracking.
    /// Returns `None` if the id is not tracked or parsing fails.
    ///
    /// Uses partial JSON repair to handle truncated JSON from streaming.
    pub fn finish(&mut self, id: &str) -> Option<crate::tool_parser::ParsedToolCall> {
        let acc = self.pending.remove(id)?;
        let args = if acc.arguments.is_empty() {
            Value::Object(serde_json::Map::new())
        } else {
            crate::tool_parser::repair_partial_json(&acc.arguments)?
        };
        Some(crate::tool_parser::ParsedToolCall {
            name: acc.name,
            args,
            id: Some(id.to_string()),
        })
    }

    /// Finish all pending tool calls, draining the stream.
    pub fn finish_all(&mut self) -> Vec<crate::tool_parser::ParsedToolCall> {
        let ids: Vec<String> = self.pending.keys().cloned().collect();
        ids.into_iter().filter_map(|id| self.finish(&id)).collect()
    }

    /// Return an iterator over pending tool calls (id, accumulator).
    pub fn pending(&self) -> impl Iterator<Item = (&String, &Accumulator)> {
        self.pending.iter()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn start_then_append_then_finish() {
        let mut stream = ToolStream::new();
        stream.start("call_1", "bash");
        stream.append("call_1", "{\"command\":");
        stream.append("call_1", "\"ls\"}");
        let call = stream.finish("call_1").unwrap();
        assert_eq!(call.name, "bash");
        assert_eq!(call.args["command"], "ls");
        assert_eq!(call.id, Some("call_1".to_string()));
    }

    #[test]
    fn finish_empty_args_defaults_to_empty_object() {
        let mut stream = ToolStream::new();
        stream.start("call_1", "noop");
        let call = stream.finish("call_1").unwrap();
        assert_eq!(call.args, serde_json::json!({}));
    }

    #[test]
    fn finish_invalid_json_returns_none() {
        let mut stream = ToolStream::new();
        stream.start("call_1", "bash");
        stream.append("call_1", "{bad");
        assert!(stream.finish("call_1").is_none());
    }

    #[test]
    fn finish_all_drains_pending() {
        let mut stream = ToolStream::new();
        stream.start("call_1", "bash");
        stream.start("call_2", "read");
        stream.append("call_1", "{\"cmd\":\"ls\"}");
        // Only finish call_1
        let one = stream.finish("call_1");
        assert!(one.is_some());
        // finish_all should return call_2
        let remaining = stream.finish_all();
        assert_eq!(remaining.len(), 1);
        assert_eq!(remaining[0].name, "read");
    }

    #[test]
    fn append_without_start_is_noop() {
        let mut stream = ToolStream::new();
        // Appending to unknown id should not panic
        stream.append("unknown", "{}");
        assert!(stream.finish("unknown").is_none());
    }

    #[test]
    fn finish_removes_from_pending() {
        let mut stream = ToolStream::new();
        stream.start("call_1", "bash");
        stream.append("call_1", "{\"cmd\":\"ls\"}");
        assert_eq!(stream.pending.len(), 1);
        stream.finish("call_1");
        assert_eq!(stream.pending.len(), 0);
    }
}
