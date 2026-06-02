use ratatui::{buffer::Buffer, layout::Rect, style::Style, text::{Line, Span}};
use serde_json;

use crate::glyphs;
use crate::components::message_list::MessageColors;

/// Render a tool call inline within an assistant message.
/// Shows ◆ marker in tool accent color + tool name + compact args.
/// Does NOT draw accent bar - caller handles bar drawing.
/// Returns number of rows rendered.
pub fn render_tool_call_inline(
    name: &str,
    args: &str,
    area: Rect,
    row: u16,
    margin_x: u16,
    content_width: u16,
    buf: &mut Buffer,
    tool_bar_color: ratatui::style::Color,
    text_muted: ratatui::style::Color,
) -> u16 {
    let compact_args = format_tool_args_compact(args);
    let content = if compact_args.is_empty() {
        format!("{} ·", name)
    } else {
        format!("{} · {}", name, compact_args)
    };

    // ◆ name · args (diamond in tool_bar_color, rest muted)
    let line = Line::from(vec![
        Span::raw(format!("{} ", glyphs::THOUGHT_MARKER)).style(Style::default().fg(tool_bar_color)),
        Span::raw(&content).style(Style::default().fg(text_muted)),
    ]);
    buf.set_line(margin_x, area.y + row, &line, content_width);

    1
}

/// Render a tool call message
pub fn render_tool_call_msg(
    name: &str,
    args: &str,
    result_preview: Option<&str>,
    area: Rect,
    buf: &mut Buffer,
    colors: &MessageColors,
    bar_color: ratatui::style::Color,
    _theme: &crate::theme::ThemeWrapper,
) -> u16 {
    let compact_args = format_tool_args_compact(args);
    let content = if compact_args.is_empty() {
        format!("{} ·", name)
    } else {
        format!("{} · {}", name, compact_args)
    };

    // Draw vertical bar at left edge (1 column wide, full height of block)
    let bar_char = '│';
    if let Some(cell) = buf.cell_mut((area.x, area.y)) {
        cell.set_char(bar_char).set_fg(bar_color);
    }

    if let Some(preview) = result_preview {
        if !preview.is_empty() {
            // ◆ name · args → preview (diamond in bar_color, rest muted except → preview)
            let line = Line::from(vec![
                Span::raw(format!("{} ", glyphs::THOUGHT_MARKER)).style(Style::default().fg(bar_color)),
                Span::raw(&content).style(Style::default().fg(colors.text_muted)),
                Span::raw(format!(" → {}", preview)).style(Style::default().fg(colors.error)),
            ]);
            buf.set_line(area.x + 1, area.y, &line, area.width - 1);
            return 1;
        }
    }

    // ◆ name · args (diamond in bar_color, rest muted)
    let line = Line::from(vec![
        Span::raw(format!("{} ", glyphs::THOUGHT_MARKER)).style(Style::default().fg(bar_color)),
        Span::raw(&content).style(Style::default().fg(colors.text_muted)),
    ]);
    buf.set_line(area.x + 1, area.y, &line, area.width - 1);
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


