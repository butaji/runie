//! Public rendering helpers for thoughts, tools, and turn state.

use ratatui::style::Color;
use ratatui::text::{Line, Span};

use crate::markdown_render::{apply_color_to_inlines, md_to_spans, MdInline, MdSpan};
use crate::theme::{
    style_agent, style_thinking, style_thought, style_timestamp, style_tool_header,
    style_tool_output, style_tool_running, style_tool_summary, style_turn_complete,
};
use runie_core::tool::{format_bytes, format_duration, format_tool_label};
use runie_util::display_width;

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

/// Render a blockquote from styled inline spans.
pub fn render_blockquote_from_spans(inlines: &[MdInline], base_color: Color) -> Vec<Line<'static>> {
    let spans = apply_color_to_inlines(inlines, base_color);
    let mut lines = Vec::new();
    let prefix = format!("{}│ ", GLYPH_INDENT);
    let prefix_width = display_width::width(&prefix);
    let content_width = 200u16; // Will be clamped by actual terminal width
    let rest_width = content_width.saturating_sub(prefix_width);

    let rows = wrap_styled_spans_for_blockquote(&spans, rest_width);
    for (i, row) in rows.iter().enumerate() {
        let line_prefix = if i == 0 { prefix.as_str() } else { "     " };
        let mut line_spans = vec![Span::styled(line_prefix.to_owned(), style_agent())];
        line_spans.extend(md_to_spans(row));
        lines.push(Line::from(line_spans).style(style_agent()));
    }
    if lines.is_empty() {
        lines.push(Line::from(format!("{}│", GLYPH_INDENT)).style(style_agent()));
    }
    lines
}

/// Wrap styled spans for blockquote rendering.
#[allow(clippy::assigning_clones, clippy::redundant_clone)]
fn wrap_styled_spans_for_blockquote(spans: &[MdSpan], max_width: u16) -> Vec<Vec<MdSpan>> {
    let mut result = Vec::new();
    let mut current_row = Vec::new();
    let mut current_width = 0u16;

    for span in spans.iter().cloned() {
        let span_width = display_width::width(&span.content);
        if current_width + span_width > max_width && !current_row.is_empty() {
            result.push(std::mem::take(&mut current_row));
            current_width = 0;
        }
        if span_width > max_width {
            // Break long span
            let mut partial = String::new();
            let mut partial_width = 0u16;
            for c in span.content.chars() {
                let char_width = display_width::width(&c.to_string());
                if partial_width + char_width > max_width && !partial.is_empty() {
                    if !current_row.is_empty() {
                        result.push(std::mem::take(&mut current_row));
                    }
                    current_row.push(MdSpan {
                        content: partial.clone(),
                        style: span.style,
                    });
                    current_width = display_width::width(&partial);
                    partial.clear();
                    partial_width = 0;
                }
                partial.push(c);
                partial_width += char_width;
            }
            if !partial.is_empty() {
                if current_width + partial_width > max_width && !current_row.is_empty() {
                    result.push(std::mem::take(&mut current_row));
                }
                current_row.push(MdSpan {
                    content: partial,
                    style: span.style,
                });
                current_width += partial_width;
            }
        } else {
            current_row.push(span);
            current_width += span_width;
        }
    }
    if !current_row.is_empty() {
        result.push(current_row);
    }
    result
}

/// Render a list item from styled spans.
#[allow(dead_code)]
pub fn render_list_item_from_spans(
    row: &[MdSpan],
    ordered: bool,
    idx: usize,
    is_first: bool,
    prefix: &str,
    ts_str: &str,
    _ts_width: u16,
) -> Line<'static> {
    let bullet = if ordered {
        format!("{}.", idx + 1)
    } else {
        "•".to_owned()
    };
    let bullet_prefix = format!("{} {}", prefix, bullet);

    let mut result_spans = vec![Span::styled(bullet_prefix, style_agent())];
    result_spans.extend(md_to_spans(row));

    // Only add timestamp to first item
    if is_first {
        result_spans.push(Span::styled(format!(" {}", ts_str), style_timestamp()));
    }

    Line::from(result_spans).style(style_agent())
}
