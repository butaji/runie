use ratatui::{buffer::Buffer, layout::Rect, style::Style, text::{Line, Span}};
use serde_json;

/// Render a tool call message
pub fn render_tool_call_msg(
    name: &str,
    args: &str,
    result: Option<&str>,
    is_error: bool,
    area: Rect,
    row: u16,
    margin_x: u16,
    _text_x: u16,
    _max_rows: u16,
    buf: &mut Buffer,
    _text_secondary: ratatui::style::Color,
    text_muted: ratatui::style::Color,
    success: ratatui::style::Color,
    error: ratatui::style::Color,
) -> u16 {
    let compact_args = format_tool_args_compact(args);
    let content = if compact_args.is_empty() {
        format!("{} ·", name)
    } else {
        format!("{} · {}", name, compact_args)
    };

    if let Some(result_text) = result {
        let result_preview = result_text.lines().next().unwrap_or("").trim();
        if !result_preview.is_empty() {
            let color = if is_error { error } else { success };
            let preview = if result_preview.len() > 40 {
                format!("{}...", &result_preview[..40])
            } else {
                result_preview.to_string()
            };
            // ◆ name · args → preview (all muted except → preview)
            let line = Line::from(vec![
                Span::raw("◆ ").style(Style::default().fg(text_muted)),
                Span::raw(&content).style(Style::default().fg(text_muted)),
                Span::raw(format!(" → {}", preview)).style(Style::default().fg(color)),
            ]);
            buf.set_line(margin_x, area.y + row, &line, area.width - margin_x + area.x - 2);
            return 1;
        }
    }

    // ◆ name · args (diamond muted, rest muted)
    let line = Line::from(vec![
        Span::raw("◆ ").style(Style::default().fg(text_muted)),
        Span::raw(&content).style(Style::default().fg(text_muted)),
    ]);
    buf.set_line(margin_x, area.y + row, &line, area.width - margin_x + area.x - 2);
    1
}

/// Format tool arguments in compact form for single-line display
pub fn format_tool_args_compact(args: &str) -> String {
    if args.is_empty() {
        return String::new();
    }

    if let Ok(json) = serde_json::from_str::<serde_json::Value>(args) {
        format_compact_from_json(&json)
    } else {
        args.trim().to_string()
    }
}

fn format_compact_from_json(json: &serde_json::Value) -> String {
    let serde_json::Value::Object(map) = json else {
        return json.to_string();
    };

    // Single-arg tools show just the value
    if map.len() == 1 {
        if let Some((_, value)) = map.iter().next() {
            return match value {
                serde_json::Value::String(s) => s.clone(),
                other => other.to_string(),
            };
        }
    }

    // Multi-arg show first two args
    let parts: Vec<String> = map.iter()
        .take(2)
        .map(|(k, v)| format!("{}={}", k, v.to_string().trim_matches('"')))
        .collect();

    if map.len() > 2 {
        format!("{}, ...", parts.join(", "))
    } else {
        parts.join(", ")
    }
}


