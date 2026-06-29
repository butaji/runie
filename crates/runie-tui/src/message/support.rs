//! Public rendering helpers for thoughts, tools, and turn state.

use ratatui::text::{Line, Span};

use crate::theme::{
    style_agent, style_thinking, style_thought, style_timestamp, style_tool_header,
    style_tool_output, style_tool_running, style_tool_summary, style_turn_complete, GLYPH_AGENT,
};
use runie_util::display_width;
use runie_core::tool::{format_bytes, format_duration, format_tool_label};

use super::{add_lr_margins, add_lr_margins_to_lines, word_wrap, GLYPH_INDENT};

pub fn render_thought_marker(content: &str, content_width: u16) -> Vec<Line<'static>> {
    let inner_width = content_width.saturating_sub(2);
    let style = style_thought();
    let mut lines: Vec<Line<'static>> = Vec::new();
    for raw_line in content.lines() {
        if raw_line.is_empty() {
            lines.push(add_lr_margins(Line::from("").style(style)));
            continue;
        }
        for chunk in word_wrap(raw_line, inner_width, inner_width) {
            lines.push(add_lr_margins(Line::from(chunk.to_string()).style(style)));
        }
    }
    if lines.is_empty() {
        lines.push(add_lr_margins(Line::from("").style(style)));
    }
    lines
}

pub fn render_thinking(started: std::time::Instant) -> Vec<Line<'static>> {
    let lines = vec![
        Line::from(crate::theme::thinking_line(started.elapsed().as_secs_f64()))
            .style(style_thinking()),
    ];
    add_lr_margins_to_lines(lines)
}

pub fn render_thought_summary(content: &str, _duration_secs: f64) -> Vec<Line<'static>> {
    let first_line = content.lines().next().unwrap_or(content);
    let lines = vec![Line::from(format!("{} [+]", first_line)).style(style_thought())];
    add_lr_margins_to_lines(lines)
}

pub fn render_tool_running(name: &str, args: &str, duration_secs: f64) -> Vec<Line<'static>> {
    let label = format_tool_label(name, args);
    let lines = vec![
        Line::from(format!("{} {} {:.1}s", "⠋", label, duration_secs)).style(style_tool_running()),
    ];
    add_lr_margins_to_lines(lines)
}

pub fn render_tool_done(
    name: &str,
    args: &str,
    duration_secs: f64,
    output: &str,
    bytes_transferred: Option<u64>,
    error: bool,
) -> Vec<Line<'static>> {
    let label = format_tool_label(name, args);
    let status_icon = if error { "✗" } else { "✓" };
    let duration = format_duration(duration_secs);
    let bytes_str = bytes_transferred
        .map(|b| format!(" ⇣{}", format_bytes(b)))
        .unwrap_or_default();
    let header = format!(
        "{} {} {} {}{}",
        status_icon,
        label,
        duration,
        bytes_str,
        if error { " [✗]" } else { "" }
    );
    let mut lines = vec![Line::from(header).style(style_tool_header())];
    if !output.is_empty() {
        if runie_core::diff::Diff::is_diff_output(output) {
            lines.extend(crate::diff::render_diff_text(output));
        } else {
            for line in output.lines() {
                lines.push(Line::from(line.to_owned()).style(style_tool_output()));
            }
        }
    }
    add_lr_margins_to_lines(lines)
}

pub fn render_tool_summary(name: &str, args: &str, duration_secs: f64) -> Vec<Line<'static>> {
    let label = format_tool_label(name, args);
    let duration = format_duration(duration_secs);
    let lines =
        vec![Line::from(format!("✓ {} {} [+]", label, duration)).style(style_tool_summary())];
    add_lr_margins_to_lines(lines)
}

pub fn render_turn_complete(duration_secs: f64) -> Vec<Line<'static>> {
    let lines = vec![
        Line::from(format!("Turn completed in {:.1}s", duration_secs)).style(style_turn_complete()),
    ];
    add_lr_margins_to_lines(lines)
}

pub fn render_context_group(tools: &[runie_core::Element], collapsed: bool) -> Vec<Line<'static>> {
    if collapsed {
        return vec![add_lr_margins(
            Line::from(context_group_summary(tools)).style(style_tool_summary()),
        )];
    }

    let mut lines = Vec::new();
    for tool in tools {
        lines.extend(render_context_tool(tool));
    }
    lines
}

fn context_group_summary(tools: &[runie_core::Element]) -> String {
    let mut counts: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
    for name in tools.iter().filter_map(tool_element_name) {
        *counts.entry(name).or_insert(0) += 1;
    }
    let mut pairs: Vec<(String, usize)> = counts.into_iter().collect();
    pairs.sort_by(|a, b| a.0.cmp(&b.0));
    let summary = pairs
        .iter()
        .map(|(name, count)| format!("{}×{}", name, count))
        .collect::<Vec<_>>()
        .join(", ");
    format!("Gathering context… {}", summary)
}

fn tool_element_name(elem: &runie_core::Element) -> Option<String> {
    match elem {
        runie_core::Element::ToolDone { name, .. }
        | runie_core::Element::ToolSummary { name, .. } => Some(name.clone()),
        _ => None,
    }
}

fn render_context_tool(elem: &runie_core::Element) -> Vec<Line<'static>> {
    match elem {
        runie_core::Element::ToolDone {
            name,
            args,
            duration_secs,
            output,
            bytes_transferred,
            error,
            ..
        } => render_tool_done(
            name,
            args,
            *duration_secs,
            output,
            *bytes_transferred,
            *error,
        ),
        runie_core::Element::ToolSummary {
            name,
            duration_secs,
            ..
        } => render_tool_summary(name, "", *duration_secs),
        _ => Vec::new(),
    }
}

pub fn render_blockquote_lines(text: &str) -> Vec<Line<'static>> {
    text.lines()
        .map(|line| {
            Line::from(format!("{}│ {}", GLYPH_INDENT, line)).style(crate::theme::style_agent())
        })
        .collect()
}

pub fn render_list_item(
    item: &str,
    ordered: bool,
    idx: usize,
    is_first: bool,
    content_width: u16,
    ts_str: &str,
) -> Line<'static> {
    let bullet = if ordered {
        format!("{}.", idx + 1)
    } else {
        "•".to_owned()
    };
    let first_line_prefix = if is_first {
        format!("{} {}", GLYPH_AGENT, bullet)
    } else {
        format!("{} {}", GLYPH_INDENT, bullet)
    };
    let rest_prefix = format!("{}   ", GLYPH_INDENT);
    let lines: Vec<&str> = item.lines().collect();
    let mut result_spans: Vec<Span<'static>> = Vec::new();
    let mut text_len = 0u16;

    for (j, line) in lines.iter().enumerate() {
        let prefix = if j == 0 {
            &first_line_prefix
        } else {
            &rest_prefix
        };
        if j > 0 {
            result_spans.push(Span::raw("\n".to_owned()));
        }
        result_spans.push(Span::styled(prefix.clone(), style_agent()));
        result_spans.push(Span::styled(line.to_string(), style_agent()));
        text_len = display_width::width(prefix) + display_width::width(line);
    }

    push_list_timestamp(&mut result_spans, is_first, content_width, ts_str, text_len);
    Line::from(result_spans).style(style_agent())
}

fn push_list_timestamp(
    spans: &mut Vec<Span<'static>>,
    is_first: bool,
    content_width: u16,
    ts_str: &str,
    text_len: u16,
) {
    if !is_first || content_width == 0 {
        return;
    }
    let ts_width = ts_str.len() as u16 + 1;
    let padding = content_width
        .saturating_sub(text_len)
        .saturating_sub(ts_width);
    if padding > 0 {
        spans.push(Span::raw(" ".repeat(padding as usize)));
    }
    spans.push(Span::styled(format!(" {}", ts_str), style_timestamp()));
}
