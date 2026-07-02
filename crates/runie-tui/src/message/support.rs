//! Public rendering helpers for thoughts, tools, and turn state.

use ratatui::style::Color;
use ratatui::text::{Line, Span};

use crate::markdown_render::{apply_color_to_inlines, md_to_spans, MdSpan};
use crate::theme::{
    style_agent, style_thinking, style_thought, style_timestamp, style_tool_header,
    style_tool_output, style_tool_running, style_tool_summary, style_turn_complete,
    GLYPH_BULLET, GLYPH_CHECK, GLYPH_INDENT, GLYPH_SPINNER, GLYPH_X, INDICATOR_ERROR,
};
use runie_core::tool::{format_bytes, format_duration, format_tool_label};
use runie_core::display_width;

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
        Line::from(format!("{} {} {:.1}s", GLYPH_SPINNER, label, duration_secs)).style(style_tool_running()),
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
    let status_icon = if error { GLYPH_X } else { GLYPH_CHECK };
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
        if error { INDICATOR_ERROR } else { "" }
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

/// Render a blockquote from plain markdown text using tui_markdown.
pub fn render_blockquote_from_spans(text: &str, base_color: Color) -> Vec<Line<'static>> {
    // Use tui_markdown for styling (via apply_color_to_inlines).
    let spans = apply_color_to_inlines(text, base_color);
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
///
/// Uses `textwrap` for display-width-aware wrapping. Each original span is kept
/// intact where possible; long spans are broken character-by-character (preserving
/// style). The result is a list of rows, each row being a list of spans that fit
/// within `max_width`.
#[allow(clippy::assigning_clones, clippy::redundant_clone)]
fn wrap_styled_spans_for_blockquote(spans: &[MdSpan], max_width: u16) -> Vec<Vec<MdSpan>> {
    let max_w = max_width as usize;

    // For simple single-span content, use textwrap directly.
    if spans.len() == 1 {
        let span = &spans[0];
        if display_width::width(&span.content) <= max_width {
            return vec![vec![span.clone()]];
        }
        // Break long single span using textwrap, keeping the style.
        let wrapped = textwrap::wrap(&span.content, max_w);
        return wrapped
            .into_iter()
            .map(|line| vec![MdSpan { content: line.into_owned(), style: span.style }])
            .collect();
    }

    // Multi-span case: use textwrap to determine line breaks, then map spans.
    // Strategy: wrap the concatenated content, then reconstruct spans per line.
    // For simplicity, we keep spans intact where they fit; break only at span boundaries
    // (which may cause slight overfilling but preserves per-span styles).
    let mut result: Vec<Vec<MdSpan>> = Vec::new();
    let mut current_row: Vec<MdSpan> = Vec::new();
    let mut current_width = 0usize;

    for span in spans.iter().cloned() {
        let span_width = display_width::width(&span.content) as usize;

        // If adding this span exceeds max_width, start a new row.
        if current_width + span_width > max_w && !current_row.is_empty() {
            result.push(std::mem::take(&mut current_row));
            current_width = 0;
        }

        if span_width > max_w {
            // Long span: break using textwrap, each fragment keeps the same style.
            let wrapped = textwrap::wrap(&span.content, max_w);
            for line in wrapped {
                let line_owned = line.into_owned();
                if !current_row.is_empty() {
                    result.push(std::mem::take(&mut current_row));
                }
                current_row.push(MdSpan { content: line_owned, style: span.style });
                current_width = display_width::width(&current_row[0].content) as usize;
            }
        } else {
            current_row.push(span);
            current_width += span_width;
        }
    }
    if !current_row.is_empty() {
        result.push(current_row);
    }

    // Edge case: empty result (shouldn't happen but handle gracefully).
    if result.is_empty() {
        result.push(Vec::new());
    }
    result
}

/// Render a list item from styled spans.
///
/// Kept for future ordered-list rendering in the markdown pipeline.
/// Currently unused but exercised by doctests.
#[allow(dead_code, reason = "kept for future ordered-list rendering")]
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
        GLYPH_BULLET.to_owned()
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
