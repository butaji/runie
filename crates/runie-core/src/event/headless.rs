//! Headless streaming event types for JSONL output.
//!
//! These events are emitted by the headless runner and serialized as
//! newline-delimited JSON to stdout. All headless modes (print, json, server)
//! share the same event vocabulary.
//!
//! Derivable from the canonical `Event` via `HeadlessEvent::try_from_event`.
//!
//! Inspired by Grok Build's headless output format.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::convert::TryFrom;

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

    /// Try to convert a canonical `Event` to a headless streaming event.
    /// Returns `None` for events that have no headless equivalent.
    pub fn try_from_event(event: &crate::Event) -> Option<Self> {
        use HeadlessEvent as H;
        match event {
            crate::Event::ResponseDelta { content, .. } if !content.is_empty() => Some(H::Text {
                data: content.clone(),
            }),
            crate::Event::ThinkingDelta { content, .. } if !content.is_empty() => {
                Some(H::Thinking {
                    data: content.clone(),
                })
            }
            crate::Event::ToolStart { id, name, .. } => Some(H::ToolCallStart {
                id: id.clone(),
                name: name.clone(),
            }),
            crate::Event::ToolInputDelta { id, content } => Some(H::ToolCallInputDelta {
                id: id.clone(),
                delta: content.clone(),
            }),
            crate::Event::ToolEnd { id, output: _, .. } => {
                // ToolEnd can be either a tool call end (streaming) or a tool result
                // (after execution). Use ToolCallEnd for streaming; ToolResult would be
                // emitted separately after execution.
                Some(H::ToolCallEnd { id: id.clone() })
            }
            crate::Event::TokenStatsUpdated {
                tokens_in,
                tokens_out,
                ..
            } => Some(H::Usage {
                input_tokens: *tokens_in,
                output_tokens: *tokens_out,
            }),
            crate::Event::Error { message, .. } => Some(H::Error {
                message: message.clone(),
            }),
            crate::Event::Done { .. } => Some(H::End {
                stop_reason: "stop".into(),
                session_id: None,
                request_id: None,
            }),
            crate::Event::PermissionRequest {
                request_id,
                tool,
                input,
            } => {
                let args = serde_json::from_value(input.clone()).unwrap_or_default();
                Some(H::PermissionRequest {
                    id: request_id.clone(),
                    tool: tool.clone(),
                    args,
                })
            }
            // All other Event variants have no headless equivalent
            _ => None,
        }
    }
}

/// Derive a headless event from a canonical `Event`.
impl TryFrom<&crate::Event> for HeadlessEvent {
    type Error = ();

    fn try_from(
        event: &crate::Event,
    ) -> Result<HeadlessEvent, <HeadlessEvent as TryFrom<&crate::Event>>::Error> {
        Self::try_from_event(event).ok_or(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── Event → HeadlessEvent ─────────────────────────────────────────────────

    #[test]
    fn headless_from_response_delta() {
        let event = crate::Event::ResponseDelta {
            id: "".into(),
            content: "Hello, ".into(),
        };
        let result = HeadlessEvent::try_from_event(&event);
        assert!(result.is_some());
        assert!(matches!(result.unwrap(), HeadlessEvent::Text { data } if data == "Hello, "));
    }

    #[test]
    fn headless_from_response_delta_ignores_empty() {
        let event = crate::Event::ResponseDelta {
            id: "".into(),
            content: String::new(),
        };
        assert!(HeadlessEvent::try_from_event(&event).is_none());
    }

    #[test]
    fn headless_from_thinking_delta() {
        let event = crate::Event::ThinkingDelta {
            id: "".into(),
            content: "thinking...".into(),
        };
        let result = HeadlessEvent::try_from_event(&event);
        assert!(result.is_some());
        assert!(
            matches!(result.unwrap(), HeadlessEvent::Thinking { data } if data == "thinking...")
        );
    }

    #[test]
    fn headless_from_tool_start() {
        let event = crate::Event::ToolStart {
            id: "c1".into(),
            name: "bash".into(),
            input: serde_json::json!({}),
        };
        let result = HeadlessEvent::try_from_event(&event);
        assert!(result.is_some());
        assert!(matches!(
            result.unwrap(),
            HeadlessEvent::ToolCallStart { id, name } if id == "c1" && name == "bash"
        ));
    }

    #[test]
    fn headless_from_tool_input_delta() {
        let event = crate::Event::ToolInputDelta {
            id: "c1".into(),
            content: "{\"cmd\": ".into(),
        };
        let result = HeadlessEvent::try_from_event(&event);
        assert!(result.is_some());
        assert!(matches!(
            result.unwrap(),
            HeadlessEvent::ToolCallInputDelta { id, delta } if id == "c1" && delta == "{\"cmd\": "
        ));
    }

    #[test]
    fn headless_from_tool_end() {
        let event = crate::Event::ToolEnd {
            id: "c1".into(),
            duration_secs: 1.0,
            output: "done".into(),

            input: None,
        };
        let result = HeadlessEvent::try_from_event(&event);
        assert!(result.is_some());
        assert!(matches!(result.unwrap(), HeadlessEvent::ToolCallEnd { id } if id == "c1"));
    }

    #[test]
    fn headless_from_token_stats() {
        let event = crate::Event::TokenStatsUpdated {
            tokens_in: 100,
            tokens_out: 50,
        };
        let result = HeadlessEvent::try_from_event(&event);
        assert!(result.is_some());
        assert!(matches!(
            result.unwrap(),
            HeadlessEvent::Usage {
                input_tokens: 100,
                output_tokens: 50
            }
        ));
    }

    #[test]
    fn headless_from_error() {
        let event = crate::Event::Error {
            id: "".into(),
            message: "rate limited".into(),
        };
        let result = HeadlessEvent::try_from_event(&event);
        assert!(result.is_some());
        assert!(matches!(
            result.unwrap(),
            HeadlessEvent::Error { message } if message == "rate limited"
        ));
    }

    #[test]
    fn headless_from_done() {
        let event = crate::Event::Done { id: "".into() };
        let result = HeadlessEvent::try_from_event(&event);
        assert!(result.is_some());
        assert!(matches!(
            result.unwrap(),
            HeadlessEvent::End { stop_reason, .. } if stop_reason == "stop"
        ));
    }

    #[test]
    fn headless_from_permission_request() {
        let event = crate::Event::PermissionRequest {
            request_id: "p1".into(),
            tool: "bash".into(),
            input: serde_json::json!({"cmd": "ls"}),
        };
        let result = HeadlessEvent::try_from_event(&event);
        assert!(result.is_some());
        assert!(matches!(
            result.unwrap(),
            HeadlessEvent::PermissionRequest { id, tool, .. }
            if id == "p1" && tool == "bash"
        ));
    }

    #[test]
    fn non_headless_events_return_none() {
        let cases: Vec<crate::Event> = vec![
            crate::Event::Quit,
            crate::Event::Input('x'),
            crate::Event::SetThinkingLevel(crate::model::ThinkingLevel::Medium),
            crate::Event::TurnComplete {
                id: "".into(),
                duration_secs: 0.0,
            },
        ];
        for event in cases {
            assert!(
                HeadlessEvent::try_from_event(&event).is_none(),
                "{:?} should not become headless",
                event
            );
        }
    }

    // ── TryFrom<&Event> for HeadlessEvent ──────────────────────────────────────

    #[test]
    fn try_from_event_ok() {
        let event = crate::Event::ResponseDelta {
            id: "".into(),
            content: "hi".into(),
        };
        let result: Result<HeadlessEvent, _> = HeadlessEvent::try_from(&event);
        assert!(result.is_ok());
    }

    #[test]
    fn try_from_event_err_for_non_headless() {
        let event = crate::Event::Quit;
        let result: Result<HeadlessEvent, _> = HeadlessEvent::try_from(&event);
        assert!(result.is_err());
    }

    // ── Existing serialization tests ────────────────────────────────────────────

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
