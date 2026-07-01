//! Blocking permission approval modal.

use ratatui::{
    text::Line,
    widgets::{Paragraph, Wrap},
    Frame,
};
use runie_core::Snapshot;

use crate::theme::style_hint;

/// Maximum length for formatted input before truncation.
const MAX_INPUT_LENGTH: usize = 500;

/// Format a tool's input arguments into a human-readable summary.
///
/// Handles common built-in tools with tool-specific formatting:
/// - `bash`: shows the command
/// - `read_file`: shows the file path
/// - `write_file`: shows the file path and content preview
/// - `edit_file`: shows the file path
/// - `list_dir`: shows the directory path
/// - `grep`/`find`: shows the pattern and path
/// - Other tools: shows key-value summary
///
/// Large inputs are truncated with "..." indicator.
pub fn format_tool_input(tool: &str, input: &serde_json::Value) -> String {
    let formatted = match (tool, input) {
        // bash: show the command
        ("bash", input) => {
            let cmd = input.get("command")
                .or_else(|| input.get("cmd"))
                .and_then(|v| v.as_str())
                .unwrap_or("<no command>");
            format!("Command: {}", cmd)
        }
        
        // read_file: show the file path
        ("read_file", input) => {
            let path = input.get("path")
                .and_then(|v| v.as_str())
                .unwrap_or("<no path>");
            format!("File: {}", path)
        }
        
        // write_file: show path and content preview
        ("write_file", input) => {
            let path = input.get("path")
                .and_then(|v| v.as_str())
                .unwrap_or("<no path>");
            let content = input.get("content")
                .or_else(|| input.get("text"))
                .and_then(|v| v.as_str())
                .map(|s| truncate_str(s, 100))
                .unwrap_or_else(|| "<no content>".to_string());
            format!("File: {} | Content: {}", path, content)
        }
        
        // edit_file: show the file path
        ("edit_file", input) => {
            let path = input.get("path")
                .or_else(|| input.get("file"))
                .and_then(|v| v.as_str())
                .unwrap_or("<no path>");
            format!("File: {}", path)
        }
        
        // list_dir: show the directory path
        ("list_dir", input) => {
            let path = input.get("path")
                .or_else(|| input.get("dir"))
                .and_then(|v| v.as_str())
                .unwrap_or(".");
            format!("Directory: {}", path)
        }
        
        // grep/find: show pattern and path
        ("grep" | "find", input) => {
            let pattern = input.get("pattern")
                .or_else(|| input.get("query"))
                .or_else(|| input.get("search"))
                .and_then(|v| v.as_str())
                .unwrap_or("<no pattern>");
            let path = input.get("path")
                .or_else(|| input.get("dir"))
                .and_then(|v| v.as_str())
                .unwrap_or(".");
            format!("Pattern: {} | Path: {}", pattern, path)
        }
        
        // fetch_docs: show URL
        ("fetch_docs", input) => {
            let url = input.get("url")
                .and_then(|v| v.as_str())
                .unwrap_or("<no url>");
            format!("URL: {}", url)
        }
        
        // search: show query
        ("search", input) => {
            let query = input.get("query")
                .or_else(|| input.get("q"))
                .and_then(|v| v.as_str())
                .unwrap_or("<no query>");
            format!("Query: {}", query)
        }
        
        // Default: show key-value pairs
        _ => {
            format_json_args(input)
        }
    };
    
    truncate_str(&formatted, MAX_INPUT_LENGTH)
}

/// Format JSON arguments as key-value pairs.
fn format_json_args(input: &serde_json::Value) -> String {
    match input {
        serde_json::Value::Object(map) => {
            let args: Vec<String> = map.iter()
                .map(|(k, v)| {
                    let value_str = match v {
                        serde_json::Value::String(s) => truncate_str(s, 50),
                        serde_json::Value::Number(n) => n.to_string(),
                        serde_json::Value::Bool(b) => b.to_string(),
                        serde_json::Value::Null => "null".to_string(),
                        serde_json::Value::Array(arr) => {
                            format!("[{} items]", arr.len())
                        }
                        serde_json::Value::Object(obj) => {
                            format!("{{{}}}", obj.keys().cloned().collect::<Vec<_>>().join(", "))
                        }
                    };
                    format!("{}: {}", k, value_str)
                })
                .collect();
            args.join(" | ")
        }
        serde_json::Value::String(s) => truncate_str(s, MAX_INPUT_LENGTH),
        _ => format!("{}", input),
    }
}

/// Truncate a string to max_len, adding "..." if truncated.
fn truncate_str(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else if max_len < 3 {
        "...".to_string()
    } else {
        format!("{}...", &s[..max_len - 3])
    }
}

/// Render a blocking modal asking the user to allow or deny a tool call.
pub fn permission_dialog(f: &mut Frame, snap: &Snapshot) {
    let request = match &snap.permission_request {
        Some(r) => r,
        None => return,
    };

    let inner = super::panel::setup_popup(f, " Permission Required ");
    let input_summary = format_tool_input(&request.tool, &request.input);
    let lines = vec![
        Line::from(format!("Tool: {}", request.tool)),
        Line::from(""),
        Line::from(format!("Details: {}", input_summary)),
        Line::from(""),
        Line::from("[y] Allow   [n] Deny   [a] Always allow").style(style_hint()),
    ];

    f.render_widget(
        Paragraph::new(lines)
            .wrap(Wrap { trim: true })
            .style(style_hint()),
        inner,
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bash_command_formatting() {
        let input = serde_json::json!({"command": "ls -la"});
        let result = format_tool_input("bash", &input);
        assert_eq!(result, "Command: ls -la");
    }

    #[test]
    fn test_read_file_formatting() {
        let input = serde_json::json!({"path": "/tmp/test.txt"});
        let result = format_tool_input("read_file", &input);
        assert_eq!(result, "File: /tmp/test.txt");
    }

    #[test]
    fn test_write_file_formatting() {
        let input = serde_json::json!({"path": "/tmp/out.txt", "content": "Hello World"});
        let result = format_tool_input("write_file", &input);
        assert!(result.contains("File: /tmp/out.txt"));
        assert!(result.contains("Content: Hello World"));
    }

    #[test]
    fn test_edit_file_formatting() {
        let input = serde_json::json!({"path": "/tmp/test.rs", "old_text": "fn foo", "new_text": "fn bar"});
        let result = format_tool_input("edit_file", &input);
        assert!(result.contains("File: /tmp/test.rs"));
    }

    #[test]
    fn test_list_dir_formatting() {
        let input = serde_json::json!({"path": "/home/user"});
        let result = format_tool_input("list_dir", &input);
        assert_eq!(result, "Directory: /home/user");
    }

    #[test]
    fn test_grep_formatting() {
        let input = serde_json::json!({"pattern": "TODO", "path": "/src"});
        let result = format_tool_input("grep", &input);
        assert!(result.contains("Pattern: TODO"));
        assert!(result.contains("Path: /src"));
    }

    #[test]
    fn test_fetch_docs_formatting() {
        let input = serde_json::json!({"url": "https://example.com/docs"});
        let result = format_tool_input("fetch_docs", &input);
        assert_eq!(result, "URL: https://example.com/docs");
    }

    #[test]
    fn test_search_formatting() {
        let input = serde_json::json!({"query": "rust async"});
        let result = format_tool_input("search", &input);
        assert_eq!(result, "Query: rust async");
    }

    #[test]
    fn test_unknown_tool_formatting() {
        let input = serde_json::json!({"arg1": "value1", "arg2": 42});
        let result = format_tool_input("mcp_tool", &input);
        assert!(result.contains("arg1: value1"));
        assert!(result.contains("arg2: 42"));
    }

    #[test]
    fn test_truncation() {
        let long_input = serde_json::json!({"command": "x".repeat(600)});
        let result = format_tool_input("bash", &long_input);
        assert!(result.ends_with("..."));
        assert!(result.len() <= MAX_INPUT_LENGTH);
    }

    #[test]
    fn test_write_file_content_truncation() {
        let long_content = serde_json::json!({
            "path": "/tmp/test.txt",
            "content": "x".repeat(200)
        });
        let result = format_tool_input("write_file", &long_content);
        // Content preview should be truncated to 100 chars
        assert!(result.len() <= MAX_INPUT_LENGTH);
    }

    #[test]
    fn test_truncate_str_edge_cases() {
        assert_eq!(truncate_str("hello", 10), "hello");
        assert_eq!(truncate_str("hello world", 8), "hello...");
        assert_eq!(truncate_str("hello world", 3), "...");
        assert_eq!(truncate_str("hello world", 0), "...");
        assert_eq!(truncate_str("hello world", 1), "...");
        assert_eq!(truncate_str("hello world", 2), "...");
    }

    #[test]
    fn test_empty_input() {
        let input = serde_json::json!({});
        let result = format_tool_input("bash", &input);
        assert!(result.contains("no command"));
    }

    #[test]
    fn test_null_values() {
        let input = serde_json::json!({"key": null, "path": "test.txt"});
        let result = format_tool_input("unknown_tool", &input);
        assert!(result.contains("key: null"));
        assert!(result.contains("path: test.txt"));
    }

    #[test]
    fn test_array_values() {
        let input = serde_json::json!({"items": [1, 2, 3, 4, 5]});
        let result = format_tool_input("unknown_tool", &input);
        assert!(result.contains("[5 items]"));
    }

    #[test]
    fn test_object_values() {
        let input = serde_json::json!({"nested": {"a": 1, "b": 2}});
        let result = format_tool_input("unknown_tool", &input);
        assert!(result.contains("nested:"));
    }
}
