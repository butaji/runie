//! Tool-stream accumulator: centralizes buffering of JSON argument fragments
//! across streaming deltas into parsed tool calls.
//!
//! Replaces duplicated `Accumulator` / `ToolCallAccumulator` logic that existed
//! in `runie-provider` (OpenAI) and `runie-agent`.

use crate::tool_parser::{repair_partial_json, ParsedToolCall};

/// Buffers name and JSON arguments for a single tool call.
#[derive(Debug, Default)]
pub struct Accumulator {
    pub name: String,
    pub arguments: String,
}

/// Accumulates streaming tool-call deltas and produces parsed tool calls on finish.
#[derive(Debug, Default)]
pub struct ToolStream {
    /// Tool-call accumulators keyed by id.
    accumulators: std::collections::HashMap<String, Accumulator>,
}

impl ToolStream {
    /// Create a new empty tool stream.
    pub fn new() -> Self {
        Self {
            accumulators: std::collections::HashMap::new(),
        }
    }

    /// Start a new tool call with the given id and name.
    pub fn start(&mut self, id: &str, name: &str) {
        self.accumulators
            .entry(id.to_string())
            .or_default()
            .name = name.to_string();
    }

    /// Append a JSON argument fragment to the tool call identified by `id`.
    /// No-op if `id` is not known.
    pub fn append(&mut self, id: &str, delta_json: &str) {
        if let Some(acc) = self.accumulators.get_mut(id) {
            acc.arguments.push_str(delta_json);
        }
    }

    /// Finish the tool call identified by `id`, returning a parsed tool call.
    /// Empty argument string defaults to `"{}"`. Returns `None` if `id` is unknown
    /// or name is empty.
    pub fn finish(&mut self, id: &str) -> Option<ParsedToolCall> {
        let acc = self.accumulators.remove(id)?;
        if acc.name.is_empty() {
            return None;
        }
        let args = repair_partial_json(&acc.arguments)?;
        Some(ParsedToolCall {
            name: acc.name,
            args,
            id: Some(id.to_string()),
        })
    }

    /// Finish all pending tool calls, returning the parsed results.
    pub fn finish_all(&mut self) -> Vec<ParsedToolCall> {
        let ids: Vec<String> = self.accumulators.keys().cloned().collect();
        let mut results = Vec::new();
        for id in ids {
            if let Some(call) = self.finish(&id) {
                results.push(call);
            }
        }
        results
    }

    /// Iterate over pending tool-call accumulators.
    pub fn pending(&self) -> impl Iterator<Item = (&String, &Accumulator)> {
        self.accumulators.iter()
    }

    // --- Index-keyed API (for OpenAI compatibility) ---

    /// Start a tool call at the given index, initially keyed by index.
    /// Returns the id if it was already set on this accumulator.
    pub fn start_by_index(&mut self, index: usize, _id: Option<&str>, name: &str) {
        let key = index.to_string();
        let acc = self.accumulators.entry(key).or_default();
        // Store name temporarily until id arrives
        if acc.name.is_empty() && !name.is_empty() {
            acc.name = name.to_string();
        }
    }

    /// Append arguments to the accumulator at the given index (creates if not exists).
    pub fn append_by_index(&mut self, index: usize, args: &str) {
        let key = index.to_string();
        self.accumulators.entry(key).or_default().arguments.push_str(args);
    }

    /// Get mutable accumulator by index.
    pub fn get_mut_by_index(&mut self, index: usize) -> Option<&mut Accumulator> {
        self.accumulators.get_mut(&index.to_string())
    }

    /// Check if id-keyed accumulator exists.
    pub fn has_by_id(&self, id: &str) -> bool {
        self.accumulators.contains_key(id)
    }

    /// Get mutable accumulator by id.
    pub fn get_mut_by_id(&mut self, id: &str) -> Option<&mut Accumulator> {
        self.accumulators.get_mut(id)
    }

    /// Finish by index, returning parsed tool call.
    pub fn finish_by_index(&mut self, index: usize) -> Option<ParsedToolCall> {
        self.finish(&index.to_string())
    }

    /// Remove accumulator by index.
    pub fn remove_by_index(&mut self, index: usize) -> Option<Accumulator> {
        self.accumulators.remove(&index.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::Value;

    #[test]
    fn tool_stream_start_then_append_then_finish() {
        let mut stream = ToolStream::new();
        stream.start("call_1", "bash");
        stream.append("call_1", "{\"command\"");
        stream.append("call_1", ":\"ls\"}");
        let call = stream.finish("call_1");
        assert!(call.is_some());
        let call = call.unwrap();
        assert_eq!(call.name, "bash");
        assert_eq!(call.args["command"], "ls");
        assert_eq!(call.id, Some("call_1".to_string()));
    }

    #[test]
    fn tool_stream_finish_empty_args_defaults_to_empty_object() {
        let mut stream = ToolStream::new();
        stream.start("call_1", "noop");
        let call = stream.finish("call_1");
        assert!(call.is_some());
        let call = call.unwrap();
        assert_eq!(call.args, Value::Object(Default::default()));
    }

    #[test]
    fn tool_stream_finish_invalid_json_returns_none() {
        let mut stream = ToolStream::new();
        stream.start("call_1", "bash");
        stream.append("call_1", "{bad");
        let call = stream.finish("call_1");
        assert!(call.is_none());
    }

    #[test]
    fn tool_stream_finish_all_drains_pending() {
        let mut stream = ToolStream::new();
        stream.start("call_1", "tool_a");
        stream.start("call_2", "tool_b");
        stream.append("call_1", "{\"x\":1}");
        // Finish only call_1
        let result = stream.finish("call_1");
        assert!(result.is_some());
        // finish_all returns remaining call_2
        let remaining = stream.finish_all();
        assert_eq!(remaining.len(), 1);
        assert_eq!(remaining[0].name, "tool_b");
    }

    #[test]
    fn tool_stream_append_without_start_is_noop() {
        let mut stream = ToolStream::new();
        // Appending to unknown id should not panic
        stream.append("unknown", "{}");
        let result = stream.finish("unknown");
        assert!(result.is_none());
        // Stream should still be usable
        stream.start("call_1", "bash");
        stream.append("call_1", "{\"cmd\":\"ls\"}");
        let call = stream.finish("call_1");
        assert!(call.is_some());
    }

    #[test]
    fn tool_stream_pending_iterates_over_accumulators() {
        let mut stream = ToolStream::new();
        stream.start("call_1", "tool_a");
        stream.start("call_2", "tool_b");
        stream.append("call_1", "{\"x\":1}");

        let pending: Vec<_> = stream.pending().collect();
        assert_eq!(pending.len(), 2);
        let call_1 = pending.iter().find(|(id, _)| *id == "call_1").unwrap();
        assert_eq!(call_1.1.name, "tool_a");
        assert_eq!(call_1.1.arguments, "{\"x\":1}");
    }

    #[test]
    fn tool_stream_finish_without_name_returns_none() {
        let mut stream = ToolStream::new();
        // Start with empty name (simulates id-only start)
        stream.start("call_1", "");
        stream.append("call_1", "{\"x\":1}");
        let call = stream.finish("call_1");
        assert!(call.is_none());
    }

    #[test]
    fn tool_stream_finish_uses_repair_for_truncated_args() {
        let mut stream = ToolStream::new();
        stream.start("call_1", "bash");
        stream.append("call_1", r#"{"command":"ls"#); // truncated: missing "}
        let call = stream.finish("call_1");
        assert!(call.is_some());
        let call = call.unwrap();
        assert_eq!(call.name, "bash");
        assert_eq!(call.args["command"], "ls");
        assert_eq!(call.id, Some("call_1".to_string()));
    }
}
