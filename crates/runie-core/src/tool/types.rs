//! Core tool-call types and partial JSON repair.
//!
//! [`ParsedToolCall`] and [`ToolParseError`] are used by the streaming state machine
//! in [`tool_stream`](crate::tool_stream) and as a text-based fallback when providers
//! do not emit structured `ProviderEvent::ToolCall*` events.

use serde_json::{Map, Value};

/// A parsed tool invocation: name, JSON arguments, and an optional call id.
#[derive(Debug, Clone, PartialEq)]
pub struct ParsedToolCall {
    pub name: String,
    pub args: Value,
    pub id: Option<String>,
}

/// A tool-call parse error: the raw line and a human-readable reason.
#[derive(Debug, Clone, PartialEq)]
pub struct ToolParseError {
    pub raw: String,
    pub reason: String,
}

/// Try to repair a partial JSON string by appending missing closing characters.
pub fn repair_partial_json(raw: &str) -> Option<Value> {
    if raw.is_empty() {
        return Some(Value::Object(Map::new()));
    }
    if let Ok(v) = serde_json::from_str(raw) {
        return Some(v);
    }
    if let Ok(v) = serde_json::from_str(&format!("{raw}}}")) {
        return Some(v);
    }
    if let Ok(v) = serde_json::from_str(&format!("{raw}\"}}")) {
        return Some(v);
    }
    if let Ok(v) = serde_json::from_str(&format!("{raw}]")) {
        return Some(v);
    }
    complete_by_brace_counting(raw)
        .and_then(|repaired| serde_json::from_str(&repaired).ok())
}

fn complete_by_brace_counting(raw: &str) -> Option<String> {
    let bytes = raw.as_bytes();
    let mut result = raw.to_owned();
    let mut stack: Vec<u8> = Vec::new();
    let mut in_string = false;
    let mut escape = false;
    for &c in bytes {
        if in_string {
            if escape {
                escape = false;
            } else if c == b'\\' {
                escape = true;
            } else if c == b'"' {
                in_string = false;
                stack.pop();
            }
        } else {
            match c {
                b'"' => {
                    in_string = true;
                    stack.push(c);
                }
                b'{' | b'[' => {
                    stack.push(c);
                }
                b'}' if stack.last() == Some(&b'{') => {
                    stack.pop();
                }
                b']' if stack.last() == Some(&b'[') => {
                    stack.pop();
                }
                _ => {}
            }
        }
    }
    while let Some(c) = stack.pop() {
        match c {
            b'"' => result.push('"'),
            b'{' => result.push('}'),
            b'[' => result.push(']'),
            _ => {}
        }
    }
    if result.len() > raw.len() {
        Some(result)
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn repair_partial_json_valid() {
        let json = r#"{"command":"ls"}"#;
        assert!(repair_partial_json(json).is_some());
    }

    #[test]
    fn repair_partial_json_missing_brace() {
        let json = r#"{"command":"ls""#;
        assert!(repair_partial_json(json).is_some());
    }

    #[test]
    fn repair_partial_json_empty() {
        assert_eq!(repair_partial_json("").unwrap(), serde_json::json!({}));
    }

    #[test]
    fn repair_partial_json_garbage() {
        assert!(repair_partial_json("not json at all").is_none());
    }
}
