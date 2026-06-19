//! Stripping helpers for tool-call artifacts.

use super::{is_tool_call_value, TOOL_CALL_END, TOOL_CALL_START};
use serde_json::Value;

/// Run the full stripping pipeline.
pub fn strip_all(content: &str) -> String {
    let without_markup = strip_tool_call_markup(content);
    let without_minimax = strip_minimax_tool_calls(&without_markup);
    let without_inline = strip_inline_json_objects(&without_minimax);
    let without_fenced = strip_inline_fenced_tools(&without_inline);
    let without_lines = strip_line_markers(&without_fenced);
    let without_empty_fences = strip_empty_code_fences(&without_lines);
    normalize_blank_lines(&without_empty_fences)
}

fn normalize_blank_lines(content: &str) -> String {
    content
        .lines()
        .filter(|line| !line.trim().is_empty())
        .collect::<Vec<_>>()
        .join("\n")
}

fn strip_tool_call_markup(content: &str) -> String {
    let mut result = String::new();
    let mut rest = content;
    while let Some(start) = rest.find(TOOL_CALL_START) {
        result.push_str(&rest[..start]);
        let after_start = &rest[start + TOOL_CALL_START.len()..];
        let Some(end) = after_start.find(TOOL_CALL_END) else {
            rest = "";
            break;
        };
        rest = &after_start[end + TOOL_CALL_END.len()..];
    }
    result.push_str(rest);
    result
}

fn strip_minimax_tool_calls(content: &str) -> String {
    const OPEN_M2: &str = "<minimax:tool_call>";
    const CLOSE_M2: &str = "</minimax:tool_call>";
    const OPEN_M3: &str = "<tool_call>";
    const CLOSE_M3: &str = "</tool_call>";
    let normalized = normalize_m3_delimiters(content);
    let mut result = String::new();
    let mut rest = normalized.as_str();
    while let Some(start) = rest.find(OPEN_M2).or_else(|| rest.find(OPEN_M3)) {
        result.push_str(&rest[..start]);
        let after_open = &rest[start..];
        let (open, close) = if after_open.starts_with(OPEN_M2) {
            (OPEN_M2, CLOSE_M2)
        } else {
            (OPEN_M3, CLOSE_M3)
        };
        let after_open = &after_open[open.len()..];
        let Some(end) = after_open.find(close) else {
            rest = "";
            break;
        };
        rest = &after_open[end + close.len()..];
    }
    result.push_str(rest);
    result
}

fn normalize_m3_delimiters(text: &str) -> String {
    let mut out = text.to_string();
    out = out.replace("]<]minimax[>[</", "</");
    out = out.replace("]<]minimax[>[<", "<");
    out
}

fn strip_inline_fenced_tools(content: &str) -> String {
    let mut result = String::with_capacity(content.len());
    for line in content.lines() {
        if let Some(idx) = line.find("```") {
            let rest = &line[idx + 3..];
            let after_lang = strip_language_prefix(rest).trim_start();
            if !after_lang.is_empty()
                && !after_lang.starts_with('{')
                && !after_lang.contains("```")
            {
                result.push_str(after_lang);
                continue;
            }
        }
        if !result.is_empty() {
            result.push('\n');
        }
        result.push_str(line);
    }
    result
}

const FENCE_LANGS: &[&str] = &[
    "bash", "diff", "html", "javascript", "js", "json", "markdown", "md", "plaintext", "py",
    "python", "rust", "sh", "shell", "sql", "text", "toml", "ts", "typescript", "xml", "yaml",
    "yml",
];

fn strip_language_prefix(rest: &str) -> &str {
    FENCE_LANGS
        .iter()
        .filter(|lang| rest.starts_with(*lang))
        .max_by_key(|lang| lang.len())
        .map(|lang| &rest[lang.len()..])
        .unwrap_or(rest)
}

fn strip_inline_json_objects(content: &str) -> String {
    let mut result = String::with_capacity(content.len());
    let mut chars = content.char_indices().peekable();
    while let Some((i, c)) = chars.next() {
        if c != '{' {
            result.push(c);
            continue;
        }
        if let Some((end, value)) = parse_json_object_at(content.as_bytes(), i) {
            if is_tool_call_value(&value) {
                while chars
                    .peek()
                    .map(|(idx, _)| *idx <= end)
                    .unwrap_or(false)
                {
                    chars.next();
                }
                continue;
            }
        }
        result.push('{');
    }
    result
}

fn parse_json_object_at(bytes: &[u8], start: usize) -> Option<(usize, Value)> {
    let end = find_object_end(bytes, start)?;
    let slice = std::str::from_utf8(&bytes[start..=end]).ok()?;
    let value: Value = serde_json::from_str(slice).ok()?;
    Some((end, value))
}

fn find_object_end(bytes: &[u8], start: usize) -> Option<usize> {
    let mut depth = 1usize;
    let mut in_string = false;
    let mut escape = false;
    let mut i = start + 1;
    while i < bytes.len() {
        let c = bytes[i];
        if in_string {
            if escape {
                escape = false;
            } else if c == b'\\' {
                escape = true;
            } else if c == b'"' {
                in_string = false;
            }
        } else if c == b'"' {
            in_string = true;
        } else if c == b'{' {
            depth += 1;
        } else if c == b'}' {
            depth -= 1;
            if depth == 0 {
                return Some(i);
            }
        }
        i += 1;
    }
    None
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
        result.trim_end().to_string()
    } else {
        result
    }
}

fn strip_empty_code_fences(content: &str) -> String {
    let mut result = String::with_capacity(content.len());
    let mut buf: Vec<String> = Vec::new();
    let mut in_fence = false;
    for line in content.lines() {
        let is_fence = line.trim_start().starts_with("```") || line.contains(" ```");
        if is_fence {
            if in_fence {
                let body: String = buf.iter().skip(1).flat_map(|s| s.chars()).collect();
                let body_trim = body.trim();
                if !body_trim.is_empty() && !is_tool_call_json(body_trim) {
                    for l in &buf {
                        if !result.is_empty() {
                            result.push('\n');
                        }
                        result.push_str(l);
                    }
                    if !result.is_empty() {
                        result.push('\n');
                    }
                    result.push_str(line);
                }
                buf.clear();
                in_fence = false;
            } else {
                buf.clear();
                buf.push(line.to_string());
                in_fence = true;
            }
            continue;
        }
        if in_fence {
            buf.push(line.to_string());
        } else {
            if !result.is_empty() {
                result.push('\n');
            }
            result.push_str(line);
        }
    }
    if in_fence {
        for l in &buf {
            if !result.is_empty() {
                result.push('\n');
            }
            result.push_str(l);
        }
    }
    result
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
        let input =
            "Here's the result:\n{\"name\": \"read_file\", \"arguments\": {\"path\": \"/test\"}}";
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
        let input = r#"before [TOOL_CALL]{tool => "bash", args => {}} after"#;
        let result = strip_all(input);
        assert_eq!(result, "before ");
    }

    #[test]
    fn test_strip_all_minimax_m3_delimiters() {
        let input = r#"I'll read it.
]<]minimax[>[<tool_call>
]<]minimax[>[<invoke name="read_file">]<]minimax[>[<path>README.md]<]minimax[>[</path>]<]minimax[>[</invoke>
]<]minimax[>[</tool_call>
Done."#;
        let result = strip_all(input);
        assert_eq!(result, "I'll read it.\nDone.");
    }

    #[test]
    fn test_strip_all_minimax() {
        let input = r#"I'll list files.
<minimax:tool_call>
<invoke name="list_dir">
<parameter name="path">.</parameter>
</invoke>
</minimax:tool_call>
Done."#;
        let result = strip_all(input);
        assert_eq!(result, "I'll list files.\nDone.");
    }

    #[test]
    fn test_strip_all_inline_json() {
        let input = r#"Here's the result: {"name": "read_file", "arguments": {"path": "/test"}} Done."#;
        let result = strip_all(input);
        assert_eq!(result, "Here's the result:  Done.");
    }

    #[test]
    fn test_strip_all_code_fenced_json() {
        let input = "→ ```json\n{\"name\": \"list_dir\", \"arguments\": {\"path\": \".\"}}\n```\nHere's the current directory.";
        let result = strip_all(input);
        assert_eq!(result, "Here's the current directory.");
    }

    #[test]
    fn test_strip_all_fenced_inline_json() {
        let input = "→ ```json{\"name\": \"list_dir\", \"arguments\": {\"path\": \".\"}}Here's the current directory.";
        let result = strip_all(input);
        assert_eq!(result, "Here's the current directory.");
    }

    #[test]
    fn test_strip_all_preserves_legitimate_json() {
        let input = r#"Example config: {"name": "foo", "arguments": {"x": 1}}."#;
        let result = strip_all(input);
        assert_eq!(result, input);
    }

    #[test]
    fn test_strip_all_preserves_code_block() {
        let input = "```json\n{\"name\": \"foo\"}\n```";
        let result = strip_all(input);
        assert_eq!(result, input);
    }
}

#[cfg(test)]
mod unicode_bug_tests {
    use super::*;

    #[test]
    fn strip_inline_json_objects_preserves_unicode() {
        let content = "hello 😊 world";
        let result = strip_inline_json_objects(content);
        assert_eq!(result, content);
    }

    #[test]
    fn strip_all_preserves_unicode() {
        let content = "café ñiño 日本";
        let result = strip_all(content);
        assert_eq!(result, content);
    }

    #[test]
    fn strip_inline_json_objects_strips_tool_call_and_preserves_unicode() {
        let content = r#"hola 😊 {"name":"bash","arguments":{"command":"ls"}} adiós 🎉"#;
        let result = strip_inline_json_objects(content);
        assert_eq!(result, "hola 😊  adiós 🎉");
    }
}
