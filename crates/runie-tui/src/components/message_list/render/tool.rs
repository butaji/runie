use ratatui::{buffer::Buffer, layout::Rect, style::Style, text::Line};
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
    text_secondary: ratatui::style::Color,
    _text_muted: ratatui::style::Color,
    success: ratatui::style::Color,
    error: ratatui::style::Color,
) -> u16 {
    let compact_args = format_tool_args_compact(args);
    let mut text = format!("● {} · {}", name, compact_args);

    if let Some(result_text) = result {
        let result_preview = result_text.lines().next().unwrap_or("").trim();
        if !result_preview.is_empty() {
            let color = if is_error { error } else { success };
            let preview = if result_preview.len() > 40 {
                format!("{}...", &result_preview[..40])
            } else {
                result_preview.to_string()
            };
            text.push_str(&format!(" → {}", preview));
            let line = Line::raw(text).style(Style::default().fg(color));
            buf.set_line(margin_x, area.y + row, &line, area.width - margin_x + area.x - 2);
            return 1;
        }
    }

    let line = Line::raw(text).style(Style::default().fg(text_secondary));
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

/// Phase 3 placeholder - tool header rendering (superseded by FeedBuilder)
#[allow(dead_code)]
pub fn draw_tool_header(
    margin_x: u16,
    text_x: u16,
    area: Rect,
    row: u16,
    buf: &mut Buffer,
    _text_muted: ratatui::style::Color,
    text_secondary: ratatui::style::Color,
    name: &str,
    args: &str,
) {
    if let Some(cell) = buf.cell_mut((margin_x, area.y + row)) {
        cell.set_char('●');
        cell.set_style(Style::default().fg(text_secondary));
    }

    let compact_args = format_tool_args_compact(args);
    let header_text = if compact_args.is_empty() {
        name.to_string()
    } else {
        format!("{} · {}", name, compact_args)
    };

    let line = Line::raw(header_text).style(Style::default().fg(text_secondary));
    buf.set_line(text_x, area.y + row, &line, area.width - 4);
}

/// Phase 3 placeholder - tool result rendering (superseded by FeedBuilder)
#[allow(dead_code)]
pub fn draw_tool_result(
    result_text: &str,
    is_error: bool,
    area: Rect,
    row: u16,
    text_x: u16,
    max_rows: u16,
    buf: &mut Buffer,
    text_muted: ratatui::style::Color,
    _success: ratatui::style::Color,
    error: ratatui::style::Color,
) -> u16 {
    let result_lines: Vec<&str> = result_text.split('\n').filter(|l| !l.is_empty()).collect();
    if result_lines.is_empty() {
        return 0;
    }

    let mut rendered = 0u16;
    let prefix = if is_error { "  └✗ " } else { "  └ " };

    for (idx, line_text) in result_lines.iter().enumerate() {
        let result_y = row + idx as u16;
        if result_y >= max_rows {
            break;
        }

        if idx == 0 {
            let prefix_text = format!("{}{}", prefix, line_text);
            let line = Line::raw(prefix_text).style(Style::default().fg(if is_error { error } else { text_muted }));
            buf.set_line(text_x, area.y + result_y, &line, area.width.saturating_sub(text_x));
        } else {
            let indented_text = format!("    {}", line_text);
            let line = Line::raw(indented_text).style(Style::default().fg(text_muted));
            buf.set_line(text_x, area.y + result_y, &line, area.width.saturating_sub(text_x));
        }
        rendered += 1;
    }

    rendered
}
