//! Public rendering helpers for thoughts, tools, and turn state.

use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};

use crate::markdown_render::{apply_color_to_inlines, md_to_spans, MdSpan};
use crate::theme::{
    blend_color, color_accent, color_bg, color_subagent_completed_bright,
    color_subagent_completed_diamond, color_subagent_failed_bright, color_subagent_failed_diamond,
    color_subagent_running_bar, color_subagent_running_diamond, color_subagent_running_dim,
    pulse_brightness, style_agent, style_feed_timestamp, style_thinking, style_thought,
    style_tool_header, style_tool_output, style_tool_running, style_tool_summary, style_turn_complete,
    wave_brightness, GLYPH_AGENT, GLYPH_BULLET, GLYPH_INDENT, GLYPH_SUBAGENT_BAR,
    GLYPH_SUBAGENT_DIAMOND, GLYPH_SUBAGENT_QUOTE_LEFT, GLYPH_SUBAGENT_QUOTE_RIGHT,
    GLYPH_SPINNER, GLYPH_X,
};
use runie_core::tool::{format_bytes, format_tool_label_parts};
use unicode_width::UnicodeWidthStr;

use super::word_wrap;

/// Display-cell width for any `AsRef<str>` type.
fn str_width(s: impl AsRef<str>) -> usize {
    UnicodeWidthStr::width(s.as_ref())
}

pub fn render_thought_marker(content: &str, content_width: u16) -> Vec<Line<'static>> {
    let style = style_thought();
    let mut lines: Vec<Line<'static>> = Vec::new();
    for raw_line in content.lines() {
        if raw_line.is_empty() {
            lines.push(Line::from("").style(style));
            continue;
        }
        for chunk in word_wrap(raw_line, content_width, content_width) {
            lines.push(Line::from(chunk.to_string()).style(style));
        }
    }
    if lines.is_empty() {
        lines.push(Line::from("").style(style));
    }
    lines
}

pub fn render_thinking(started: std::time::Instant) -> Vec<Line<'static>> {
    vec![
        Line::from(crate::theme::thinking_line(started.elapsed().as_secs_f64()))
            .style(style_thinking()),
    ]
}

pub fn render_thought_summary(content: &str, _duration_secs: f64) -> Vec<Line<'static>> {
    let style = style_thought();
    let first_line = content.lines().next().unwrap_or(content);
    // Grok-style summary: `◆ ` + bold "Thought" + plain " for Xs", all dim.
    // No [+] affordance — expandability is advertised in the hint bar.
    match first_line.strip_prefix("◆ ") {
        Some(rest) => match rest.split_once(' ') {
            Some((word, tail)) => vec![Line::from(vec![
                Span::styled(GLYPH_AGENT, style),
                Span::styled(word.to_owned(), style.bold()),
                Span::styled(format!(" {tail}"), style),
            ])],
            None => vec![Line::from(vec![
                Span::styled(GLYPH_AGENT, style),
                Span::styled(rest.to_owned(), style.bold()),
            ])],
        },
        None => vec![Line::from(first_line.to_owned()).style(style)],
    }
}

/// Animation speed for running blocks (radians per tick).
/// ~0.15 gives a nice smooth wave that travels the block in ~40 ticks (grok parity).
const WAVE_SPEED: f32 = 0.15;

/// Number of rows per wave cycle (grok parity).
const WAVE_ROWS: u16 = 32;

pub fn render_tool_running(name: &str, args: &str, duration_secs: f64, animation_frame: u32) -> Vec<Line<'static>> {
    let (verb, args_part) = format_tool_label_parts(name, args);
    // Use wave_brightness on the spinner glyph color for running tools (grok parity).
    // The wave travels through the glyph row using WAVE_ROWS phase offset.
    let wave = wave_brightness(animation_frame, 0, WAVE_ROWS, WAVE_SPEED);
    let base_style = style_tool_running();
    let spinner_color = blend_color(color_bg(), color_accent(), wave)
        .unwrap_or_else(|| color_accent());
    vec![Line::from(vec![
        Span::styled(GLYPH_SPINNER.to_string(), Style::new().fg(spinner_color)),
        Span::styled(" ", base_style),
        Span::styled(verb, base_style.bold()),
        Span::styled(args_part, base_style),
        Span::styled(format!(" {:.1}s", duration_secs), base_style),
    ])]
}

/// Grok-style finish-flash: linear decay over 400ms after tool completion.
/// Returns brightness in [0.0, 1.0] — 1.0 = peak flash (just finished),
/// 0.0 = settled (flash done).
/// `finished_at` is the monotonic Instant when the tool finished.
fn finish_flash(finished_at: &Option<std::time::Instant>, _animation_frame: u32) -> f32 {
    const FLASH_DURATION_MS: f64 = 400.0;

    let Some(finished) = finished_at else {
        return 0.0;
    };
    let elapsed_ms = finished.elapsed().as_secs_f64() * 1000.0;
    if elapsed_ms >= FLASH_DURATION_MS {
        return 0.0;
    }
    // Linear decay: 1.0 → 0.0 over FLASH_DURATION_MS
    (1.0 - elapsed_ms / FLASH_DURATION_MS) as f32
}

pub fn render_tool_done(
    name: &str,
    args: &str,
    _duration_secs: f64,
    output: &str,
    bytes_transferred: Option<u64>,
    error: bool,
    finished_at: &Option<std::time::Instant>,
    animation_frame: u32,
) -> Vec<Line<'static>> {
    let (verb, args_part) = format_tool_label_parts(name, args);
    let bytes_str = bytes_transferred
        .map(|b| format!(" ⇣{}", format_bytes(b)))
        .unwrap_or_default();
    let base_style = style_tool_header();

    // Grok-style finish-flash: blend accent toward bg at peak, then settle.
    // The flash uses the same wave brightness function as tool-running.
    let flash = finish_flash(finished_at, animation_frame);
    let glyph_style = if flash > 0.0 {
        // At peak flash, blend accent toward bg, creating a bright flash.
        let glyph_color = blend_color(color_bg(), color_accent(), 0.3 + flash * 0.7)
            .unwrap_or_else(color_accent);
        Style::new().fg(glyph_color)
    } else {
        base_style
    };

    // Grok-style done post: `◆ ` + bold verb/name + plain args, all dim.
    // No ✓, no trailing duration. Errors keep the ✗ marker.
    let glyph = if error {
        format!("{GLYPH_X} ")
    } else {
        GLYPH_AGENT.to_string()
    };
    let mut spans = vec![
        Span::styled(glyph, glyph_style),
        Span::styled(verb, base_style.bold()),
    ];
    let tail = format!("{args_part}{bytes_str}");
    if !tail.is_empty() {
        spans.push(Span::styled(tail, base_style));
    }
    let mut lines = vec![Line::from(spans)];
    if !output.is_empty() {
        if runie_core::diff::Diff::is_diff_output(output) {
            lines.extend(crate::diff::render_diff_text(output));
        } else {
            for line in output.lines() {
                lines.push(Line::from(line.to_owned()).style(style_tool_output()));
            }
        }
    }
    lines
}

pub fn render_tool_summary(name: &str, args: &str, _duration_secs: f64) -> Vec<Line<'static>> {
    let (verb, args_part) = format_tool_label_parts(name, args);
    let style = style_tool_summary();
    let mut spans = vec![
        Span::styled(GLYPH_AGENT, style),
        Span::styled(verb, style.bold()),
    ];
    if !args_part.is_empty() {
        spans.push(Span::styled(args_part, style));
    }
    vec![Line::from(spans)]
}

pub fn render_turn_complete(duration_secs: f64) -> Vec<Line<'static>> {
    vec![
        Line::from(format!("Turn completed in {:.1}s.", duration_secs))
            .style(style_turn_complete()),
    ]
}

/// Render a swarm subagent lifecycle row (GROK.md §26).
///
/// Running:   `❙  ◆ Subagent running: “<desc>” — <activity> (<model>)`
/// Completed: `◆ Subagent completed in Xs: “<desc>”`
/// Failed:    `◆ Subagent failed in Xs: “<desc>”`
///
/// Expanded finished rows render the worker output indented under the row,
/// styled like an expanded thought body.
pub fn render_subagent_row(elem: &runie_core::Element, animation_frame: u32) -> Vec<Line<'static>> {
    let runie_core::Element::SubagentRow {
        description,
        model,
        status,
        started: _,
        duration_ms,
        activity,
        output,
        expanded,
        ..
    } = elem
    else {
        return vec![Line::from("")];
    };
    use runie_core::model::PatternWorkerStatus as S;

    let dim = style_tool_running();
    let header = match status {
        S::Running => {
            let activity_text = if activity.is_empty() { "Running" } else { activity };
            // Pulse the bar/diamond toward background using pulse_brightness (grok parity)
            let pulse = pulse_brightness(animation_frame, 0.08);
            let bar_color = blend_color(color_bg(), color_subagent_running_bar(), pulse)
                .unwrap_or(color_subagent_running_bar());
            let diamond_color = blend_color(color_bg(), color_subagent_running_diamond(), pulse)
                .unwrap_or(color_subagent_running_diamond());
            let dim_color = blend_color(color_bg(), color_subagent_running_dim(), pulse)
                .unwrap_or(color_subagent_running_dim());
            Line::from(vec![
                Span::styled(GLYPH_SUBAGENT_BAR, Style::new().fg(bar_color)),
                Span::styled("  ", Style::new().fg(bar_color)),
                Span::styled(GLYPH_SUBAGENT_DIAMOND, Style::new().fg(diamond_color)),
                Span::styled(" ", Style::new().fg(dim_color)),
                Span::styled("Subagent running: ", dim.bold()),
                Span::styled(
                    format!(
                        "{GLYPH_SUBAGENT_QUOTE_LEFT}{description}{GLYPH_SUBAGENT_QUOTE_RIGHT} — {activity_text} ({model})"
                    ),
                    dim,
                ),
            ])
        }
        S::Completed => Line::from(vec![
            Span::styled(GLYPH_SUBAGENT_DIAMOND, Style::new().fg(color_subagent_completed_diamond())),
            Span::styled(" ", Style::new().fg(color_subagent_completed_bright())),
            Span::styled(
                format!(
                    "Subagent completed in {}: {GLYPH_SUBAGENT_QUOTE_LEFT}{description}{GLYPH_SUBAGENT_QUOTE_RIGHT}",
                    runie_core::labels::format_elapsed_secs(duration_ms.unwrap_or(0) as f64 / 1000.0)
                ),
                dim,
            ),
        ]),
        S::Failed => Line::from(vec![
            Span::styled(GLYPH_SUBAGENT_DIAMOND, Style::new().fg(color_subagent_failed_diamond())),
            Span::styled(" ", Style::new().fg(color_subagent_failed_bright())),
            Span::styled(
                format!(
                    "Subagent failed in {}: {GLYPH_SUBAGENT_QUOTE_LEFT}{description}{GLYPH_SUBAGENT_QUOTE_RIGHT}",
                    runie_core::labels::format_elapsed_secs(duration_ms.unwrap_or(0) as f64 / 1000.0)
                ),
                dim,
            ),
        ]),
        S::Cancelled => Line::from(vec![
            Span::styled(GLYPH_SUBAGENT_DIAMOND, Style::new().fg(color_subagent_failed_diamond())),
            Span::styled(" ", Style::new().fg(color_subagent_failed_bright())),
            Span::styled(
                format!(
                    "Subagent cancelled in {}: {GLYPH_SUBAGENT_QUOTE_LEFT}{description}{GLYPH_SUBAGENT_QUOTE_RIGHT}",
                    runie_core::labels::format_elapsed_secs(duration_ms.unwrap_or(0) as f64 / 1000.0)
                ),
                dim,
            ),
        ]),
    };

    let mut lines = vec![header];
    if *expanded && !output.is_empty() {
        for line in output.lines() {
            lines.push(Line::from(format!("{GLYPH_INDENT}{line}")).style(style_thought()));
        }
    }
    lines
}

pub fn render_context_group(tools: &[runie_core::Element], collapsed: bool) -> Vec<Line<'static>> {
    if collapsed {
        return vec![Line::from(context_group_summary(tools)).style(style_tool_summary())];
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
            &None,
            0,
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
    let prefix_width = str_width(&prefix) as u16;
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
        if str_width(&span.content) as u16 <= max_width {
            return vec![vec![span.clone()]];
        }
        // Break long single span using textwrap, keeping the style.
        let wrapped = textwrap::wrap(&span.content, max_w);
        return wrapped
            .into_iter()
            .map(|line| {
                vec![MdSpan {
                    content: line.into_owned(),
                    style: span.style,
                }]
            })
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
        let span_width = str_width(&span.content);

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
                current_row.push(MdSpan {
                    content: line_owned,
                    style: span.style,
                });
                current_width = str_width(&current_row[0].content);
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
#[allow(clippy::too_many_arguments, dead_code, reason = "kept for future ordered-list rendering")]
pub fn render_list_item_from_spans(
    row: &[MdSpan],
    ordered: bool,
    idx: usize,
    is_first: bool,
    prefix: &str,
    ts_str: &str,
    ts_width: u16,
    content_width: u16,
) -> Line<'static> {
    let bullet = if ordered {
        format!("{}.", idx + 1)
    } else {
        GLYPH_BULLET.to_owned()
    };
    let bullet_prefix = if prefix.is_empty() {
        bullet
    } else {
        format!("{} {}", prefix, bullet)
    };
    let bullet_width = str_width(&bullet_prefix);

    let mut result_spans = vec![Span::styled(bullet_prefix, style_agent())];
    result_spans.extend(md_to_spans(row));

    // Only add timestamp to first item with proper padding
    if is_first {
        let text_width: usize = result_spans[1..]
            .iter()
            .map(|s| str_width(&s.content))
            .sum();
        let padding = content_width
            .saturating_sub(bullet_width as u16)
            .saturating_sub(text_width as u16)
            .saturating_sub(ts_width);
        if padding > 0 {
            result_spans.push(Span::raw(" ".repeat(padding as usize)));
        }
        result_spans.push(Span::styled(format!(" {}", ts_str), style_feed_timestamp()));
    }

    Line::from(result_spans).style(style_agent())
}
