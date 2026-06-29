//! Headless streaming event types for JSONL output.
//!
//! These events are emitted by the headless runner and serialized as
//! newline-delimited JSON to stdout. All headless modes (print, json, server)
//! share the same event vocabulary.
//!
//! Inspired by Grok Build's headless output format.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// A headless streaming event — serializable to JSONL.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "data")]
#[serde(rename_all = "snake_case")]
pub enum HeadlessEvent {
    /// Text delta from the assistant.
    Text { data: String },

    /// Thinking/reasoning delta (if supported).
    Thinking { data: String },

    /// A tool call started.
    ToolCallStart { id: String, name: String },

    /// A delta of tool input JSON.
    ToolCallInputDelta { id: String, delta: String },

    /// A tool call finished.
    ToolCallEnd { id: String },

    /// Permission was requested for a tool.
    PermissionRequest {
        id: String,
        tool: String,
        args: HashMap<String, serde_json::Value>,
    },

    /// Result of a tool execution.
    ToolResult { id: String, output: String },

    /// Token usage for the turn.
    Usage {
        input_tokens: usize,
        output_tokens: usize,
    },

    /// An error occurred.
    Error { message: String },

    /// Turn finished.
    End {
        stop_reason: String,
        session_id: Option<String>,
        request_id: Option<String>,
    },
}

impl HeadlessEvent {
    /// Serialize to a JSONL line.
    pub fn to_json_line(&self) -> String {
        serde_json::to_string(self).unwrap_or_else(|e| {
            format!(r#"{{"type":"error","data":{{"message":"serialization failed: {e}"}}}}"#)
        })
    }

    /// Emit to stdout as a JSONL line (adds newline).
    pub fn print_json_line(&self) {
        let line = self.to_json_line();
        println!("{}", line);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn text_event_serialization() {
        let evt = HeadlessEvent::Text {
            data: "Hello".into(),
        };
        let line = evt.to_json_line();
        assert!(line.contains(r#""type":"text""#));
        assert!(line.contains(r#""data":"Hello""#));
    }

    #[test]
    fn tool_call_event_serialization() {
        let evt = HeadlessEvent::ToolCallStart {
            id: "c1".into(),
            name: "bash".into(),
        };
        let line = evt.to_json_line();
        assert!(line.contains(r#""type":"tool_call_start""#));
        assert!(line.contains(r#""id":"c1""#));
        assert!(line.contains(r#""name":"bash""#));
    }

    #[test]
    fn end_event_serialization() {
        let evt = HeadlessEvent::End {
            stop_reason: "EndTurn".into(),
            session_id: None,
            request_id: None,
        };
        let line = evt.to_json_line();
        assert!(line.contains(r#""type":"end""#));
        assert!(line.contains(r#""stop_reason":"EndTurn""#));
    }

    #[test]
    fn error_event_round_trips() {
        let evt = HeadlessEvent::Error {
            message: "something broke".into(),
        };
        let line = evt.to_json_line();
        let parsed: HeadlessEvent = serde_json::from_str(&line).unwrap();
        assert!(matches!(parsed, HeadlessEvent::Error { message } if message == "something broke"));
    }

    #[test]
    fn usage_event_has_correct_fields() {
        let evt = HeadlessEvent::Usage {
            input_tokens: 100,
            output_tokens: 50,
        };
        let line = evt.to_json_line();
        let parsed: HeadlessEvent = serde_json::from_str(&line).unwrap();
        assert!(matches!(
            parsed,
            HeadlessEvent::Usage {
                input_tokens: 100,
                output_tokens: 50
            }
        ));
    }

    #[test]
    fn permission_request_round_trips() {
        let mut args = HashMap::new();
        args.insert("cmd".into(), serde_json::json!("ls -la"));
        let evt = HeadlessEvent::PermissionRequest {
            id: "p1".into(),
            tool: "bash".into(),
            args,
        };
        let line = evt.to_json_line();
        let parsed: HeadlessEvent = serde_json::from_str(&line).unwrap();
        assert!(matches!(
            parsed,
            HeadlessEvent::PermissionRequest { id, tool, .. }
            if id == "p1" && tool == "bash"
        ));
    }
}
