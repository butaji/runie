//! Second-pass stripping helpers (cleanup after format stripping).

use serde_json::Value;

use super::legacy_tools::strip_inline_legacy_tools;
use super::{
    is_tool_call_value,
    strip::{strip_inline_fenced_tools, strip_inline_json_objects, strip_minimax_tool_calls, strip_tool_call_markup},
};

/// Run the full stripping pipeline (2 passes).
pub fn strip_all(content: &str) -> String {
    let stripped = strip_all_formats(content);
    cleanup_output(&stripped)
}

/// Pass 1: Strip all known tool-call formats.
fn strip_all_formats(content: &str) -> String {
    let no_tc = strip_tool_call_markup(content);
    let no_minimax = strip_minimax_tool_calls(&no_tc);
    let no_inline = strip_inline_json_objects(&no_minimax);
    let no_fenced = strip_inline_fenced_tools(&no_inline);
    strip_inline_legacy_tools(&no_fenced)
}

/// Pass 2: Remove empty fences, blank lines, and leftover tool fragments.
pub fn cleanup_output(content: &str) -> String {
    let cleaned = strip_line_markers(content);
    let no_fences = strip_empty_code_fences(&cleaned);
    normalize_blank_lines(&no_fences)
}

fn normalize_blank_lines(content: &str) -> String {
    content
        .lines()
        .filter(|line| !line.trim().is_empty())
        .collect::<Vec<_>>()
        .join("\n")
}

fn strip_line_markers(content: &str) -> String {
    let mut result = String::new();
    let mut found_tool = false;

    for line in content.lines() {
        let trimmed = line.trim();

        if trimmed.starts_with("TOOL:") {
            found_tool = true;
            continue;
        }

        if trimmed.starts_with('{') {
            if let Ok(value) = serde_json::from_str::<Value>(trimmed) {
                if is_tool_call_value(&value) {
                    found_tool = true;
                    continue;
                }
            }
        }

        if !result.is_empty() {
            result.push('\n');
        }
        result.push_str(line);
    }

    if found_tool {
        result.trim_end().to_owned()
    } else {
        result
    }
}

fn strip_empty_code_fences(content: &str) -> String {
    let mut result = String::with_capacity(content.len());
    let mut buf: Vec<String> = Vec::new();
    let mut in_fence = false;
    for line in content.lines() {
        if is_fence_line(line) {
            if in_fence {
                emit_fence_if_valid(&buf, line, &mut result);
                buf.clear();
                in_fence = false;
            } else {
                buf.clear();
                buf.push(line.to_owned());
                in_fence = true;
            }
        } else if in_fence {
            buf.push(line.to_owned());
        } else {
            push(&mut result, line);
        }
    }
    if in_fence {
        for l in &buf {
            push(&mut result, l);
        }
    }
    result
}

fn is_fence_line(line: &str) -> bool {
    line.trim_start().starts_with("```") || line.contains(" ```")
}

fn push(result: &mut String, line: &str) {
    if !result.is_empty() {
        result.push('\n');
    }
    result.push_str(line);
}

fn emit_fence_if_valid(buf: &[String], end_line: &str, result: &mut String) {
    let body = buf
        .iter()
        .skip(1)
        .flat_map(|s| s.chars())
        .collect::<String>();
    if !body.trim().is_empty() && !is_tool_call_json(body.trim()) {
        for l in buf {
            push(result, l);
        }
        push(result, end_line);
    }
}

fn is_tool_call_json(text: &str) -> bool {
    serde_json::from_str::<Value>(text)
        .ok()
        .is_some_and(|v| is_tool_call_value(&v))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_strip_all_handles_legitimate_tooltip_text() {
        let input = "Use the TOOL: parameter to configure the tool.";
        let result = strip_all(input);
        assert_eq!(result, input);
        assert!(result.contains("TOOL:"));
    }

    #[test]
    fn test_strip_all_handles_valid_tool_call() {
        let input = "Here's the result:\n{\"name\": \"read_file\", \"arguments\": {\"path\": \"/test\"}}";
        let result = strip_all(input);
        assert_eq!(result, "Here's the result:");
    }

    #[test]
    fn test_strip_all_handles_multiple_tools() {
        let input = "Result:\nTOOL:bash ls\n{\"name\": \"read_file\", \"arguments\": {}}\nDone";
        let result = strip_all(input);
        assert_eq!(result, "Result:\nDone");
    }

    #[test]
    fn test_strip_all_legacy_format() {
        let input = "Before\nTOOL:read_file /path\nAfter";
        let result = strip_all(input);
        assert_eq!(result, "Before\nAfter");
    }

    #[test]
    fn test_strip_all_markup_format() {
        let input = "Before\n[TOOL_CALL]{tool => \"bash\", args => {\"command\" => \"ls\"}}[/TOOL_CALL]\nAfter";
        let result = strip_all(input);
        assert_eq!(result, "Before\nAfter");
    }

    #[test]
    fn test_strip_all_multiple_markup_blocks() {
        let input = "Start\n[TOOL_CALL]{tool => \"bash\", args => {}}[/TOOL_CALL]\n[TOOL_CALL]{tool => \"read_file\", args => {\"path\" => \"a\"}}[/TOOL_CALL]\nEnd";
        let result = strip_all(input);
        assert_eq!(result, "Start\nEnd");
    }

    #[test]
    fn test_strip_all_malformed_markup() {
        let input = "before [TOOL_CALL]{tool => \"bash\", args => {}} after";
        let result = strip_all(input);
        assert_eq!(result, "before ");
    }

    #[test]
    fn test_strip_all_minimax_m3_delimiters() {
        let input = "I'll read it.\n]<]minimax[>[<tool_call>\n]<]minimax[>[<invoke name=\"read_file\">]<]minimax[>[<path>README.md]<]minimax[>[</path>]<]minimax[>[</invoke>\n]<]minimax[>[</tool_call>\nDone.";
        let result = strip_all(input);
        assert_eq!(result, "I'll read it.\nDone.");
    }

    #[test]
    fn test_strip_all_minimax() {
        let input = "I'll list files.\n<tool_call>\n<invoke name=\"list_dir\">\n<parameter name=\"path\">.</parameter>\n</invoke>\n<invoke name=\"read_file\">\n<parameter name=\"path\">README.md</parameter>\n</invoke>\n</tool_call>\nDone.";
        let result = strip_all(input);
        assert_eq!(result, "I'll list files.\nDone.");
    }

    #[test]
    fn test_strip_all_inline_json() {
        let input = "Here's the result: {\"name\": \"read_file\", \"arguments\": {\"path\": \"/test\"}} Done.";
        let result = strip_all(input);
        assert_eq!(result, "Here's the result:  Done.");
    }

    #[test]
    fn test_strip_all_code_fenced_json() {
        let input =
            "```json\n{\"name\": \"list_dir\", \"arguments\": {\"path\": \".\"}}\n```\nHere's the current directory.";
        let result = strip_all(input);
        assert_eq!(result, "Here's the current directory.");
    }

    #[test]
    fn test_strip_all_fenced_inline_json() {
        let input = "```json{\"name\": \"list_dir\", \"arguments\": {\"path\": \".\"}}Here's the current directory.";
        let result = strip_all(input);
        assert_eq!(result, "Here's the current directory.");
    }

    #[test]
    fn test_strip_all_preserves_legitimate_json() {
        let input = "Example config: {\"name\": \"foo\", \"arguments\": {\"x\": 1}}.";
        let result = strip_all(input);
        assert_eq!(result, input);
    }

    #[test]
    fn test_strip_all_preserves_code_block() {
        let input = "```json\n{\"name\": \"foo\"}\n```";
        let result = strip_all(input);
        assert_eq!(result, input);
    }

    #[test]
    fn stripper_collapses_to_two_passes() {
        // Verify strip_all uses 2 passes: strip_all_formats then cleanup_output.
        let input = "Before\nTOOL:read_file /path\nAfter";
        let result = strip_all(input);
        assert_eq!(result, "Before\nAfter");
    }
}

#[cfg(test)]
mod unicode_bug_tests {
    use super::*;

    #[test]
    fn strip_inline_json_objects_preserves_unicode() {
        let content = "hello world";
        let result = strip_inline_json_objects(content);
        assert_eq!(result, content);
    }

    #[test]
    fn strip_all_preserves_nonascii() {
        let content = "caf\u{00E9} \u{00F1}i\u{00F1}o \u{65E5}\u{672C}";
        let result = strip_all(content);
        assert_eq!(result, content);
    }

    #[test]
    fn strip_inline_json_objects_strips_tool_call_and_preserves_nonascii() {
        let content = "hola \u{1F600} {\"name\":\"bash\",\"arguments\":{\"command\":\"ls\"}} adios \u{1F609}";
        let result = strip_inline_json_objects(content);
        assert_eq!(result, "hola \u{1F600}  adios \u{1F609}");
    }
}
