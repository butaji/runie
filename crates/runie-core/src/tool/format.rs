//! Tool formatting and path helpers.

use super::{ToolOutput, ToolStatus};
use bytesize::ByteSize;
use humantime::format_duration as humantime_fmt;

/// Locate an executable on PATH using the `which` crate.
pub fn which_tool(name: &str) -> Option<String> {
    which::which(name)
        .ok()
        .map(|p| p.to_string_lossy().into_owned())
}

/// Async version of [`which_tool`].
///
/// Note: `which::which` is sync but fast; we wrap it in blocking to avoid
/// blocking the async executor.
pub async fn which_tool_async(name: &str) -> Option<String> {
    let name = name.to_owned();
    tokio::task::spawn_blocking(move || which_tool(&name))
        .await
        .ok()
        .flatten()
}

/// Build a standard error (or warning) [`ToolOutput`].
///
/// The `is_warning` flag reports success semantics while still surfacing the
/// message, which is useful for recoverable failures such as "no matches found".
pub fn tool_error(
    tool_name: &str,
    msg: &str,
    start: std::time::Instant,
    is_warning: bool,
) -> ToolOutput {
    ToolOutput {
        tool_name: tool_name.to_owned(),
        tool_args: serde_json::Value::Null,
        content: msg.to_owned(),
        bytes_transferred: None,
        duration: start.elapsed(),
        status: if is_warning {
            ToolStatus::Success
        } else {
            ToolStatus::Error
        },
    }
}

// ─── Inline Tool Rendering Helpers ─────────────────────────────────────────────

/// Maximum display width for tool arguments before truncation.
const ARGS_TRUNCATE_WIDTH: usize = 40;

/// Truncate args to a maximum display width, appending '…' if truncated.
pub(crate) fn truncate_args(args: &str) -> String {
    let display_width = crate::display_width::width(args) as usize;
    if display_width <= ARGS_TRUNCATE_WIDTH {
        return args.to_owned();
    }

    let mut result = String::new();
    let mut current_width = 0usize;
    for c in args.chars() {
        let char_width = crate::display_width::width(&c.to_string()) as usize;
        if current_width + char_width > ARGS_TRUNCATE_WIDTH - 1 {
            result.push('…');
            break;
        }
        result.push(c);
        current_width += char_width;
    }
    result
}

/// Format a tool label with args, truncated if needed.
///
/// Examples:
/// - `format_tool_label("bash", "echo hi")` → `"Run bash 'echo hi'"`
/// - `format_tool_label("ls", "")` → `"Run ls"`
/// - `format_tool_label("bash", "a very long command...")` → `"Run bash 'a very long comma…'"`
pub fn format_tool_label(name: &str, args: &str) -> String {
    let args = truncate_args(args);
    if args.is_empty() {
        format!("Run {}", name)
    } else {
        format!("Run {} '{}'", name, args)
    }
}

/// Extract a compact display argument from a tool-call JSON value.
///
/// Prefers the `path` or `command` field, then any string value, then falls
/// back to an empty string so the rendered label stays short.
pub fn compact_json_args(args: &serde_json::Value) -> String {
    match args {
        serde_json::Value::Object(map) => map
            .get("path")
            .or_else(|| map.get("command"))
            .and_then(|v| v.as_str())
            .map(String::from)
            .or_else(|| map.values().find_map(|v| v.as_str().map(String::from)))
            .unwrap_or_default(),
        serde_json::Value::String(s) => s.clone(),
        _ => String::new(),
    }
}

/// Format bytes into human-readable form using the `bytesize` crate.
///
/// Uses `bytesize` for the underlying numeric formatting and strips the `B`
/// suffix and trailing space to produce the same compact format as the
/// previous hand-rolled implementation (e.g. `"1.0k"`, `"3.5M"`).
/// An explicit unit override at the 1-MiB boundary preserves the
/// decimal-grouping behavior of the original implementation.
///
/// Examples:
/// - `format_bytes(567)` → `"567"`
/// - `format_bytes(1234)` → `"1.2k"`
/// - `format_bytes(3_456_789)` → `"3.5M"`
pub fn format_bytes(bytes: u64) -> String {
    if bytes < 1000 {
        return bytes.to_string();
    }
    let formatted = ByteSize(bytes).to_string();
    // bytesize v1.x auto-scales to the smallest unit ≥ 1000. Override the
    // 1-MiB boundary so 1_000_000 formats as "1.0M" (matching the original).
    // Only the KB path uses bytesize (and needs lowercasing); MB/GB are direct.
    if bytes < 1_000_000 {
        // bytesize output: "X.X KB" or "X.X MB" — strip "B" + space, lowercase unit.
        formatted
            .replace(' ', "")
            .trim_end_matches('B')
            .chars()
            .map(|c| match c {
                'K' | 'M' | 'G' => c.to_ascii_lowercase(),
                _ => c,
            })
            .collect()
    } else if bytes < 1_000_000_000 {
        let mb = bytes as f64 / 1_000_000.0;
        format!("{:.1}M", mb)
    } else {
        let gb = bytes as f64 / 1_000_000_000.0;
        format!("{:.1}G", gb)
    }
}

/// Format duration in seconds.
///
/// For sub-minute durations uses the same custom formatting as before
/// (one decimal place, `Xs` suffix). For longer durations delegates to
/// `humantime::format_duration` and strips spaces to produce the same
/// compact format (e.g. `"1m5s"`).
///
/// Examples:
/// - `format_duration(12.3)` → `"12.3s"`
/// - `format_duration(65.0)` → `"1m5s"`
pub fn format_duration(secs: f64) -> String {
    if secs < 60.0 {
        format!("{:.1}s", secs)
    } else {
        humantime_fmt(std::time::Duration::from_secs_f64(secs))
            .to_string()
            .chars()
            .filter(|c| *c != ' ')
            .collect()
    }
}

/// Truncate tool output to the configured byte and line limits.
///
/// Applies both limits and appends `"…"` when truncated. Multi-byte
/// character boundaries are preserved.
pub fn truncate_output(output: &str, max_bytes: usize, max_lines: usize) -> String {
    if output.len() <= max_bytes && output.lines().count() <= max_lines {
        return output.to_owned();
    }

    let mut truncated = truncate_to_bytes(output, max_bytes);
    truncated = truncate_to_lines(&truncated, max_lines);
    truncated
}

fn truncate_to_bytes(s: &str, max_bytes: usize) -> String {
    if s.len() <= max_bytes {
        return s.to_owned();
    }
    let mut end = max_bytes;
    while end > 0 && !s.is_char_boundary(end) {
        end -= 1;
    }
    format!("{}…", &s[..end])
}

fn truncate_to_lines(s: &str, max_lines: usize) -> String {
    let count = s.lines().count();
    if count <= max_lines {
        return s.to_owned();
    }
    let kept: Vec<&str> = s.lines().take(max_lines).collect();
    format!("{}\n…", kept.join("\n"))
}

/// Build the inline status line for a tool block.
///
/// Used by rendering tests to verify the header line format.
///
/// Examples:
/// - Running: `"⠋ Run ls . 1.8s"`
/// - Done with bytes: `"✓ Run ls . 5.7s ⇣21.2k"`
/// - Done with error: `"✗ Run bash 0.5s [✗]"`
pub fn tool_status_line(
    label: &str,
    duration_secs: f64,
    bytes: Option<u64>,
    status: &str,
) -> String {
    let dur = format_duration(duration_secs);
    let bytes_str = bytes
        .map(|b| format!(" ⇣{}", format_bytes(b)))
        .unwrap_or_default();
    let error_suffix = if status == "✗" { " [✗]" } else { "" };
    format!("{}{} {}{}{}", status, label, dur, bytes_str, error_suffix)
}
