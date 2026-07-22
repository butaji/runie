//! Legacy tool-marker stripping helpers shared by both strip passes.

use crate::tool::parse::is_known_tool;

/// Strip legacy `TOOL:bash` style markers.
pub fn strip_inline_legacy_tools(content: &str) -> String {
    let mut result = String::with_capacity(content.len());
    let mut rest = content;
    while let Some(idx) = rest.find("TOOL:") {
        let is_line_start = idx == 0 || rest.as_bytes().get(idx - 1) == Some(&b'\n');
        if !is_line_start {
            let after = &rest[idx + 5..];
            if let Some(len) = legacy_tool_marker_len(after) {
                let end = idx + 5 + len;
                if rest[end..].trim().is_empty() {
                    result.push_str(&rest[..idx]);
                    rest = "";
                    continue;
                }
            }
        }
        result.push_str(&rest[..idx + 5]);
        rest = &rest[idx + 5..];
    }
    result.push_str(rest);
    result
}

/// Returns the length of a legacy TOOL: tool name + args, or None if not a valid marker.
pub fn legacy_tool_marker_len(after: &str) -> Option<usize> {
    let trimmed = after.trim_start();
    let leading = after.len() - trimmed.len();
    let (name, consumed) = if trimmed.contains(':') {
        let parts: Vec<&str> = trimmed.splitn(3, ':').collect();
        let name = parts[0];
        if name.is_empty() || !is_known_tool(name) {
            return None;
        }
        let arg1 = parts.get(1).unwrap_or(&"");
        let arg2 = parts.get(2).unwrap_or(&"");
        let consumed = name.len() + 1 + arg1.len() + if arg2.is_empty() { 0 } else { 1 + arg2.len() };
        (name, consumed)
    } else {
        let mut tokens = trimmed.split_whitespace();
        let name = tokens.next()?;
        if !is_known_tool(name) {
            return None;
        }
        let first = tokens.next().unwrap_or("");
        let rest = tokens.collect::<Vec<_>>().join(" ");
        let consumed = name.len()
            + if first.is_empty() { 0 } else { 1 + first.len() }
            + if rest.is_empty() { 0 } else { 1 + rest.len() };
        (name, consumed)
    };
    let _ = name;
    Some(leading + consumed)
}
