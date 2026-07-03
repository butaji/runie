//! First-pass stripping helpers (tool-call format detection and removal).
//!
//! Format stripping removes tool-call artifacts in various formats:
//!   - Markup: `[TOOL_CALL]...[/TOOL_CALL]`
//!   - MiniMax M2: `</minimax:tool_call>`
//!   - MiniMax M3: `<tool_call>...</tool_call>`
//!   - Inline JSON: `{"name": "bash", "arguments": {...}}`
//!   - Fenced code: ` ```json {"name": "bash"} ``` `
//!   - Legacy: `TOOL:bash ls`
//!
//! The second pass (`strip_cleanup`) removes empty fences, blank lines,
//! and leftover markers.

use super::{is_tool_call_value, TOOL_CALL_END, TOOL_CALL_START};
use serde_json::Value;

// Re-export the full pipeline from strip_cleanup.
pub use super::strip_cleanup::strip_all;

pub(crate) fn strip_tool_call_markup(content: &str) -> String {
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

pub(crate) fn strip_minimax_tool_calls(content: &str) -> String {
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
    let mut out = text.to_owned();
    out = out.replace("]<]minimax[>[</", "</");
    out = out.replace("]<]minimax[>[<", "<");
    out
}

pub(crate) fn strip_inline_fenced_tools(content: &str) -> String {
    let mut result = String::with_capacity(content.len());
    for line in content.lines() {
        if let Some(idx) = line.find("```") {
            let rest = &line[idx + 3..];
            let after_lang = strip_language_prefix(rest).trim_start();
            if !after_lang.is_empty() && !after_lang.starts_with('{') && !after_lang.contains("```")
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
    "bash",
    "diff",
    "html",
    "javascript",
    "js",
    "json",
    "markdown",
    "md",
    "plaintext",
    "py",
    "python",
    "rust",
    "sh",
    "shell",
    "sql",
    "text",
    "toml",
    "ts",
    "typescript",
    "xml",
    "yaml",
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

pub(crate) fn strip_inline_json_objects(content: &str) -> String {
    let mut result = String::with_capacity(content.len());
    let mut chars = content.char_indices().peekable();
    while let Some((i, c)) = chars.next() {
        if c != '{' {
            result.push(c);
            continue;
        }
        if let Some((end, value)) = parse_json_object_at(content.as_bytes(), i) {
            if is_tool_call_value(&value) {
                while chars.peek().map(|(idx, _)| *idx <= end).unwrap_or(false) {
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
