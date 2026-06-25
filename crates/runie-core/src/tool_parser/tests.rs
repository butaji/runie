//! Tests for partial JSON repair functionality.

use super::*;

#[test]
fn repair_valid_json_passes_through() {
    let json = r#"{"command":"ls"}"#;
    let result = repair_partial_json(json);
    assert!(result.is_some());
    assert_eq!(
        result.unwrap().as_object().unwrap().get("command").unwrap().as_str().unwrap(),
        "ls"
    );
}

#[test]
fn repair_missing_closing_brace() {
    let json = r#"{"command":"ls""#;
    let result = repair_partial_json(json);
    assert!(result.is_some());
    let obj = result.unwrap().as_object().unwrap();
    assert_eq!(obj.get("command").unwrap().as_str().unwrap(), "ls");
}

#[test]
fn repair_missing_closing_quote_and_brace() {
    let json = r#"{"command":"ls"#;
    let result = repair_partial_json(json);
    assert!(result.is_some());
    let obj = result.unwrap().as_object().unwrap();
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
    let obj = result.unwrap().as_object().unwrap();
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
    // This tests the brace-counting with escaped quotes
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
