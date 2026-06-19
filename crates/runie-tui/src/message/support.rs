//! Public rendering helpers for thoughts, tools, and turn state.

use ratatui::text::{Line, Span};

use crate::theme::{
    style_agent, style_thinking, style_thought, style_tool_header, style_tool_output,
    style_tool_running, style_tool_summary, style_turn_complete,
};
use runie_core::tool::{format_bytes, format_duration, format_tool_label};

use super::{add_lr_margins, add_lr_margins_to_lines, word_wrap};

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
        if crate::diff::is_diff_output(output) {
            lines.extend(crate::diff::render_diff_text(output));
        } else {
            for line in output.lines() {
                lines.push(Line::from(line.to_string()).style(style_tool_output()));
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

pub fn render_context_group(
    tools: &[runie_core::Element],
    collapsed: bool,
) -> Vec<Line<'static>> {
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
        } => render_tool_done(name, args, *duration_secs, output, *bytes_transferred, *error),
        runie_core::Element::ToolSummary { name, duration_secs, .. } => {
            render_tool_summary(name, "", *duration_secs)
        }
        _ => Vec::new(),
    }
}

pub fn render_blockquote_lines(text: &str) -> Vec<Line<'static>> {
    text.lines()
        .map(|line| Line::from(format!("│ {}", line)).style(crate::theme::style_agent()))
        .collect()
}

pub fn render_list_item(
    item: &str,
    ordered: bool,
    idx: usize,
    _is_first: bool,
    _content_width: u16,
    _ts_str: &str,
) -> Line<'static> {
    let bullet = if ordered {
        format!("{}.", idx + 1)
    } else {
        "•".to_string()
    };
    let first_line_prefix = format!("{} ", bullet);
    let rest_prefix = "   ".to_string();
    let lines: Vec<&str> = item.lines().collect();
    let mut result_spans: Vec<Span<'static>> = Vec::new();

    for (j, line) in lines.iter().enumerate() {
        let prefix = if j == 0 {
            &first_line_prefix
        } else {
            &rest_prefix
        };
        if j > 0 {
            result_spans.push(Span::raw("\n".to_string()));
        }
        result_spans.push(Span::styled(prefix.clone(), style_agent()));
        result_spans.push(Span::styled(line.to_string(), style_agent()));
    }

    Line::from(result_spans).style(style_agent())
}
