//! Partial JSON repair for streaming tool-call arguments.
//!
//! When tool calls are parsed from streaming LLM responses, the JSON arguments
//! may be truncated at chunk boundaries. This module provides repair strategies
//! to recover valid JSON from partial input.

use serde_json::{Map, Value};

/// Try to repair a partial JSON string by appending missing closing characters.
///
/// Strategies tried in order:
/// 1. Already valid JSON
/// 2. Missing closing brace `}`
/// 3. Missing closing quote and brace `"}`
/// 4. Missing closing bracket `]`
/// 5. Brace-counting completion (for nested structures)
///
/// Returns `None` if all strategies fail. Empty string returns `Some({})`.
pub fn repair_partial_json(raw: &str) -> Option<Value> {
    if raw.is_empty() {
        return Some(Value::Object(Map::new()));
    }

    // Strategy 1: Already valid
    if let Ok(v) = serde_json::from_str(raw) {
        return Some(v);
    }

    // Strategy 2: Missing closing brace
    if let Ok(v) = serde_json::from_str(&format!("{raw}}}")) {
        return Some(v);
    }

    // Strategy 3: Missing closing quote and brace (common for trailing string value)
    if let Ok(v) = serde_json::from_str(&format!("{raw}\"}}")) {
        return Some(v);
    }

    // Strategy 4: Missing closing bracket (for arrays)
    if let Ok(v) = serde_json::from_str(&format!("{raw}]")) {
        return Some(v);
    }

    // Strategy 5: Brace-counting completion
    if let Some(repaired) = complete_by_brace_counting(raw) {
        if let Ok(v) = serde_json::from_str(&repaired) {
            return Some(v);
        }
    }

    None
}

/// Complete JSON by counting unmatched braces, brackets, and quotes.
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
                b'}' => {
                    if stack.last() == Some(&b'{') {
                        stack.pop();
                    }
                }
                b']'
                    if stack.last() == Some(&b'[') => {
                        stack.pop();
                    }
                _ => {}
            }
        }
    }

    // Close unmatched braces in reverse order
    while let Some(c) = stack.pop() {
        match c {
            b'"' => result.push('"'),
            b'{' => result.push('}'),
            b'[' => result.push(']'),
            _ => {}
        }
    }

    // Only return if we actually added something
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
    fn repair_valid_json_passes_through() {
        let json = r#"{"command":"ls"}"#;
        let result = repair_partial_json(json);
        assert!(result.is_some());
        let binding = result.unwrap();
        let obj = binding.as_object().unwrap();
        assert_eq!(obj.get("command").unwrap().as_str().unwrap(), "ls");
    }

    #[test]
    fn repair_missing_closing_brace() {
        let json = r#"{"command":"ls""#;
        let result = repair_partial_json(json);
        assert!(result.is_some());
        let binding = result.unwrap();
        let obj = binding.as_object().unwrap();
        assert_eq!(obj.get("command").unwrap().as_str().unwrap(), "ls");
    }

    #[test]
    fn repair_missing_closing_quote_and_brace() {
        let json = r#"{"command":"ls"#;
        let result = repair_partial_json(json);
        assert!(result.is_some());
        let binding = result.unwrap();
        let obj = binding.as_object().unwrap();
        assert_eq!(obj.get("command").unwrap().as_str().unwrap(), "ls");
    }

    #[test]
    fn repair_missing_closing_bracket() {
        let json = r#"{"files":["a","b""#;
        let result = repair_partial_json(json);
        assert!(result.is_some());
    }

    #[test]
    fn repair_empty_string_defaults_to_empty_object() {
        let result = repair_partial_json("");
        assert!(result.is_some());
        assert_eq!(result.unwrap(), serde_json::json!({}));
    }

    #[test]
    fn repair_nested_unclosed() {
        let json = r#"{"a":{"b":1"#;
        let result = repair_partial_json(json);
        assert!(result.is_some());
        let binding = result.unwrap();
        let obj = binding.as_object().unwrap();
        let nested = obj.get("a").unwrap().as_object().unwrap();
        assert_eq!(nested.get("b").unwrap().as_i64().unwrap(), 1);
    }

    #[test]
    fn repair_garbage_returns_none() {
        let result = repair_partial_json("not json at all");
        assert!(result.is_none());
    }

    #[test]
    fn repair_string_with_escaped_quotes() {
        let json = r#"{"cmd":"hello world""#;
        let result = repair_partial_json(json);
        // Should repair: missing closing }
        assert!(result.is_some());
    }

    #[test]
    fn tool_stream_finish_uses_repair_for_truncated_args() {
        use crate::tool_stream::ToolStream;
        let mut stream = ToolStream::new();
        stream.start("call_1", "bash");
        stream.append("call_1", r#"{"command":"ls"#); // truncated - missing closing brace
        let call = stream.finish("call_1");
        assert!(call.is_some());
        let call = call.unwrap();
        assert_eq!(call.name, "bash");
        assert_eq!(call.args["command"], "ls");
    }
}
