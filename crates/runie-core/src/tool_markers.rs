//! Tool marker parsing utilities.
//!
//! Provides functions to detect and strip tool call markers from text content.
//! This module unifies the logic between runie-core and runie-agent to prevent
//! drift and handle edge cases properly.

use serde_json::Value;

/// Checks if the given text contains any tool call markers.
///
/// A tool call marker is either:
/// - A line starting with "TOOL:" (legacy format)
/// - A JSON object with both "name" and "arguments" fields (structured format)
///
/// Note: This function is conservative - it may return true for text that
/// looks like a tool call but isn't. Use `parse_tool_calls` for precise parsing.
pub fn has_tool_markers(text: &str) -> bool {
    !parse_tool_calls(text).is_empty()
}

/// Parses tool calls from text content.
///
/// Returns a list of tool names found in the text.
/// A tool call is identified by:
/// - Legacy: Line starting with "TOOL:" followed by tool name
/// - Structured: JSON with "name" and "arguments" fields
pub fn parse_tool_calls(text: &str) -> Vec<String> {
    let mut tools = Vec::new();
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        
        // Check for legacy TOOL: format
        if let Some(rest) = line.strip_prefix("TOOL:") {
            let name = rest.split_whitespace().next().unwrap_or("");
            if !name.is_empty() {
                tools.push(name.to_string());
            }
            continue;
        }
        
        // Check for structured JSON format
        if line.starts_with('{') {
            if let Ok(value) = serde_json::from_str::<Value>(line) {
                if value.get("name").is_some() && value.get("arguments").is_some() {
                    if let Some(name) = value.get("name").and_then(|v| v.as_str()) {
                        tools.push(name.to_string());
                    }
                }
            }
        }
    }
    tools
}

/// Strips tool call markers from text content.
///
/// Returns the text with all tool call markers removed.
/// This preserves legitimate text that happens to contain "TOOL:" as a word.
pub fn strip_tool_markers(content: &str) -> String {
    let mut result = String::new();
    let mut found_tool = false;
    
    for line in content.lines() {
        let trimmed = line.trim();
        
        // Check for legacy TOOL: format
        if trimmed.starts_with("TOOL:") {
            found_tool = true;
            continue;
        }
        
        // Check for structured JSON tool call
        if trimmed.starts_with('{') {
            if let Ok(value) = serde_json::from_str::<Value>(trimmed) {
                if value.get("name").is_some() && value.get("arguments").is_some() {
                    found_tool = true;
                    continue;
                }
            }
        }
        
        // Keep this line
        if !result.is_empty() {
            result.push('\n');
        }
        result.push_str(line);
    }
    
    // If we found tools, trim trailing whitespace from result
    if found_tool {
        result.trim_end().to_string()
    } else {
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_has_tool_markers_positive() {
        // Legacy format
        assert!(has_tool_markers("TOOL:read_file /path/to/file"));
        
        // Structured format
        assert!(has_tool_markers(r#"{"name": "read_file", "arguments": {"path": "/test"}}"#));
        
        // Multiple tools
        assert!(has_tool_markers("Some text\nTOOL:bash ls\nAnother tool"));
    }

    #[test]
    fn test_has_tool_markers_negative() {
        // Regular text
        assert!(!has_tool_markers("Hello, this is regular text."));
        
        // JSON without name/arguments
        assert!(!has_tool_markers(r#"{"foo": "bar"}"#));
        
        // JSON with only name (not a valid tool)
        assert!(!has_tool_markers(r#"{"name": "not_a_tool"}"#));
        
        // JSON with only arguments
        assert!(!has_tool_markers(r#"{"arguments": {}}"#));
    }

    #[test]
    fn test_strip_tool_markers_handles_legitimate_tooltip_text() {
        // "TOOL:" as part of legitimate text should NOT be stripped
        let input = "Use the TOOL: parameter to configure the tool.";
        let result = strip_tool_markers(input);
        assert_eq!(result, input);
        assert!(result.contains("TOOL:"));
    }

    #[test]
    fn test_strip_tool_markers_handles_valid_tool_call() {
        // JSON tool call should be stripped
        let input = "Here's the result:\n{\"name\": \"read_file\", \"arguments\": {\"path\": \"/test\"}}";
        let result = strip_tool_markers(input);
        assert_eq!(result, "Here's the result:");
    }

    #[test]
    fn test_strip_tool_markers_handles_multiple_tools() {
        // Multiple tool calls should all be stripped
        let input = "Result:\nTOOL:bash ls\n{\"name\": \"read_file\", \"arguments\": {}}\nDone";
        let result = strip_tool_markers(input);
        assert_eq!(result, "Result:\nDone");
    }

    #[test]
    fn test_strip_tool_markers_legacy_format() {
        // Only the TOOL: line is stripped, not lines after it
        let input = "Before\nTOOL:read_file /path\nAfter";
        let result = strip_tool_markers(input);
        assert_eq!(result, "Before\nAfter");
    }

    #[test]
    fn test_parse_tool_calls() {
        let input = "TOOL:bash ls\n{\"name\": \"read_file\", \"arguments\": {\"path\": \"/test\"}}";
        let tools = parse_tool_calls(input);
        assert_eq!(tools, vec!["bash", "read_file"]);
    }

    #[test]
    fn test_parse_tool_calls_invalid_json() {
        // Invalid JSON should not be parsed as tool
        let input = "{invalid json}";
        let tools = parse_tool_calls(input);
        assert!(tools.is_empty());
    }
}
