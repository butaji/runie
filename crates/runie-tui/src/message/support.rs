//! Public rendering helpers for thoughts, tools, and turn state.

use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};

use crate::markdown_render::{apply_color_to_inlines, md_to_spans, MdSpan};
use crate::theme::{
    blend_color, color_bg, color_subagent_completed_bright, color_subagent_completed_diamond,
    color_subagent_failed_bright, color_subagent_failed_diamond, color_subagent_running_bar,
    color_subagent_running_diamond, color_subagent_running_dim, pulse_brightness, style_agent, style_feed_timestamp,
    style_thinking, style_thought, style_tool_header, style_tool_output, style_tool_running, style_tool_summary,
    style_turn_complete, GLYPH_AGENT, GLYPH_BULLET, GLYPH_INDENT, GLYPH_SUBAGENT_BAR, GLYPH_SUBAGENT_DIAMOND,
    GLYPH_SUBAGENT_QUOTE_LEFT, GLYPH_SUBAGENT_QUOTE_RIGHT, RAIL_GLYPH,
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

pub fn render_thinking() -> Vec<Line<'static>> {
    // Grok keeps the block text stable. Its shared feed compositor owns the
    // accent/bullet animation; this renderer must not emit chrome.
    vec![Line::from("Thinking…").style(style_thinking())]
}

/// Number of thought body lines to show in truncated (default) mode.
const THOUGHT_TRUNCATED_LINES: usize = 3;

/// Ellipsis line shown between header and last N truncated lines.
const ELLIPSIS_LINE: &str = "  …";

pub fn render_thought_summary(content: &str, _duration_secs: f64) -> Vec<Line<'static>> {
    let style = style_thought();
    let first_line = content.lines().next().unwrap_or(content);
    // Grok-style summary: bold "Thought" + plain " for Xs", all dim.
    // Truncated default: if body lines > THOUGHT_TRUNCATED_LINES, show
    // header + `…` + last THOUGHT_TRUNCATED_LINES lines.
    let header = match first_line.strip_prefix("◆ ") {
        Some(rest) => match rest.split_once(' ') {
            Some((word, tail)) => vec![Line::from(vec![
                Span::styled(word.to_owned(), style.bold()),
                Span::styled(format!(" {tail}"), style),
            ])],
            None => vec![Line::from(vec![
                Span::styled(rest.to_owned(), style.bold()),
            ])],
        },
        None => vec![Line::from(first_line.to_owned()).style(style)],
    };

    // Collect body lines (everything after the first line).
    let body_lines: Vec<&str> = content.lines().skip(1).collect();
    if body_lines.is_empty() {
        return header;
    }

    // Truncated default mode: show `…` + last THOUGHT_TRUNCATED_LINES body lines.
    let mut lines = header;
    if body_lines.len() > THOUGHT_TRUNCATED_LINES {
        lines.push(Line::from(ELLIPSIS_LINE).style(style));
        for line in body_lines.iter().rev().take(THOUGHT_TRUNCATED_LINES).rev() {
            let styled = if line.is_empty() {
                Line::from("").style(style)
            } else {
                Line::from(format!("  {line}")).style(style)
            };
            lines.push(styled);
        }
    } else {
        // Fewer than threshold — show all body lines.
        for line in &body_lines {
            let styled = if line.is_empty() {
                Line::from("").style(style)
            } else {
                Line::from(format!("  {line}")).style(style)
            };
            lines.push(styled);
        }
    }
    lines
}

pub fn render_tool_running(name: &str, args: &str, _duration_secs: f64, _animation_frame: u32) -> Vec<Line<'static>> {
    let (verb, args_part) = feed_tool_label_parts(name, args);
    let base_style = style_tool_running();
    vec![Line::from(vec![
        Span::styled(verb, base_style.bold()),
        Span::styled(args_part, base_style),
    ])]
}

#[allow(clippy::too_many_arguments)]
pub fn render_tool_done(
    name: &str,
    args: &str,
    _duration_secs: f64,
    output: &str,
    bytes_transferred: Option<u64>,
    _error: bool,
    _finished_at: &Option<std::time::Instant>,
    _animation_frame: u32,
) -> Vec<Line<'static>> {
    let (verb, args_part) = feed_tool_done_label_parts(name, args, output);
    let bytes_str = bytes_transferred
        .map(|b| format!(" ⇣{}", format_bytes(b)))
        .unwrap_or_default();
    let base_style = style_tool_header();

    let mut spans = vec![
        Span::styled(verb, base_style.bold()),
    ];
    let tail = format!("{args_part}{bytes_str}");
    if !tail.is_empty() {
        spans.push(Span::styled(tail, base_style));
    }
    let mut lines = vec![Line::from(spans)];
    if !output.is_empty() {
        // Blank separator line before output panel (Grok parity).
        lines.push(Line::from(""));
        // Tool output panel: bg_dark background across all output rows to content width.
        let output_style = style_tool_output().bg(crate::theme::color_code_bg());
        // Grok 2+…+3 truncation: show first 2 + … + last 3 for long outputs.
        if runie_core::diff::Diff::is_diff_output(output) {
            lines.extend(crate::diff::render_diff_text(output));
        } else {
            let output_lines: Vec<&str> = output.lines().collect();
            if output_lines.len() > 5 {
                for line in output_lines.iter().take(2) {
                    lines.push(Line::from(line.to_string()).style(output_style));
                }
                let hidden = output_lines.len() - 5;
                lines.push(Line::from(format!("… +{hidden} lines")).style(output_style));
                for line in output_lines.iter().rev().take(3).rev() {
                    lines.push(Line::from(line.to_string()).style(output_style));
                }
            } else {
                for line in &output_lines {
                    lines.push(Line::from(line.to_string()).style(output_style));
                }
            }
        }
    }
    lines
}

pub fn render_tool_summary(name: &str, args: &str, _duration_secs: f64) -> Vec<Line<'static>> {
    let (verb, args_part) = feed_tool_label_parts(name, args);
    let style = style_tool_summary();
    let mut spans = vec![Span::styled(GLYPH_AGENT, style), Span::styled(verb, style.bold())];
    if !args_part.is_empty() {
        spans.push(Span::styled(args_part, style));
    }
    vec![Line::from(spans)]
}

pub fn render_turn_complete(duration_secs: f64) -> Vec<Line<'static>> {
    vec![Line::from(format!("Worked for {:.1}s.", duration_secs)).style(style_turn_complete())]
}

/// Render Grok-style compact system/session text without assistant or tool
/// glyphs. System messages stay muted and wrap to the feed content width.
pub fn render_system_message(content: &str, content_width: u16) -> Vec<Line<'static>> {
    let style = style_turn_complete();
    let width = content_width.max(1);
    let mut lines = Vec::new();
    for raw in content.lines() {
        let wrapped = word_wrap(raw, width, width);
        if wrapped.is_empty() {
            lines.push(Line::from("").style(style));
        } else {
            lines.extend(wrapped.into_iter().map(|line| Line::from(line.to_string()).style(style)));
        }
    }
    if lines.is_empty() {
        lines.push(Line::from("").style(style));
    }
    lines
}

pub fn render_context_info(model: &str, used: usize, total: usize, turns: usize, tool_calls: usize, width: u16) -> Vec<Line<'static>> {
    let pct = if total == 0 { 0.0 } else { used as f64 / total as f64 * 100.0 };
    let short = |n: usize| if n >= 1_000_000 { format!("{:.1}m", n as f64 / 1_000_000.0) } else if n >= 1_000 { format!("{:.1}k", n as f64 / 1_000.0) } else { n.to_string() };
    // Grok's context block uses a 100-cell bar arranged as five rows of 20.
    // Runie currently has only aggregate usage, so cells represent used/free
    // capacity rather than Grok's richer per-category breakdown.
    let bar_used = ((pct / 100.0) * 100.0).round().min(100.0) as usize;
    let narrow = width < 50;
    let row_len = if narrow { 10 } else { 20 };
    let row_count = 100 / row_len;
    let bar_lines = (0..row_count)
        .map(|row| {
            let used_in_row = bar_used.saturating_sub(row * row_len).min(row_len);
            Line::from(format!("{}{}", "◆ ".repeat(used_in_row), "◇ ".repeat(row_len - used_in_row)))
                .style(style_tool_summary())
        })
        .collect::<Vec<_>>();
    let free = total.saturating_sub(used);
    let mut lines = vec![
        Line::from("Context").style(style_tool_summary().bold()),
        Line::from(format!("{} / {} tokens ({:.1}%)", short(used), short(total), pct)).style(style_tool_summary()),
        Line::from(model.to_owned()).style(style_tool_summary()),
    ];
    lines.extend(bar_lines);
    lines.extend([
        Line::from(format!("Auto-compact at 85% · ~{} tokens remaining", short(free))).style(style_tool_summary()),
        Line::from(format!("Turns: {turns} · Tool calls: {tool_calls}")).style(style_tool_summary()),
    ]);
    lines
}

/// Render Grok's non-foldable credit-limit warning card in the feed.
pub fn render_credit_limit(heading: &str, action: &str, url: &str) -> Vec<Line<'static>> {
    let heading_style = Style::default().fg(crate::theme::color_warning()).bold();
    let muted = style_turn_complete();
    let body = match action {
        "increase_payg_limit" => "You can continue by increasing your spending limit.",
        "purchase_credits" => "You can continue by purchasing more credits.",
        _ => "You can continue by enabling pay-as-you-go usage.",
    };
    vec![
        Line::from(Span::styled(heading.to_owned(), heading_style)),
        Line::from(""),
        Line::from(Span::styled(body, muted)),
        Line::from(Span::styled(url.to_owned(), Style::default().fg(crate::theme::color_agent_text()))),
    ]
}

/// Render Grok's collapsed workflow lifecycle row with its phase trail.
pub fn render_workflow(
    name: &str,
    objective: &str,
    status: &str,
    phases: &[String],
    active_agents: u32,
    duration_secs: f64,
) -> Vec<Line<'static>> {
    let style = style_tool_summary();
    let verb = match status {
        "done" | "completed" => format!("{name} done in {}: ", format_elapsed(duration_secs)),
        "failed" => format!("{name} failed in {}: ", format_elapsed(duration_secs)),
        "cancelled" => format!("{name} ◌ cancelled after {}: ", format_elapsed(duration_secs)),
        "paused" => format!("{name} paused at {}: ", format_elapsed(duration_secs)),
        _ => format!("{name}: "),
    };
    let trail = phases
        .iter()
        .map(|phase| {
            let (state, title) = phase.split_once(':').unwrap_or(("pending", phase));
            let mark = match state {
                "done" => "✓",
                "active" => "●",
                _ => "○",
            };
            format!("{title} {mark}")
        })
        .collect::<Vec<_>>()
        .join(" · ");
    let mut text = format!("Workflow {verb}{objective}");
    if !trail.is_empty() {
        text.push_str(&format!("  [{trail}]"));
    }
    if status == "running" && active_agents > 0 {
        text.push_str(&format!("  ({active_agents} agents)"));
    }
    vec![Line::from(text).style(style)]
}

fn format_elapsed(seconds: f64) -> String {
    runie_core::labels::format_elapsed_secs(seconds)
}

/// Render Grok's collapsed background-task lifecycle row.
pub fn render_background_task(
    command: &str,
    status: &str,
    description: Option<&str>,
    duration_secs: f64,
    exit_code: Option<i32>,
    signal: Option<&str>,
) -> Vec<Line<'static>> {
    let style = style_tool_summary();
    let display = description.filter(|text| !text.trim().is_empty()).unwrap_or(command).replace('\n', " ");
    let signal_is_kill = signal.is_some_and(|value| matches!(value, "killed" | "SIGTERM" | "SIGKILL" | "oom"));
    let elapsed = runie_core::labels::format_turn_timer(std::time::Duration::from_secs_f64(duration_secs.max(0.0)));
    let (verb, suffix) = match status {
        "completed" => ("completed", format!(" in {elapsed}")),
        "failed" if signal_is_kill => {
            ("killed", format!(" in {elapsed}"))
        }
        "failed" => {
            ("failed", format!(" in {elapsed}"))
        }
        "killed" | "cancelled" => ("killed", format!(" in {elapsed}")),
        _ => ("started", String::new()),
    };
    let detail = if status == "failed" && !signal_is_kill {
        signal
            .map(|s| format!(" ({s})"))
            .or_else(|| exit_code.map(|code| format!(" (exit {code})")))
            .unwrap_or_default()
    } else {
        String::new()
    };
    vec![Line::from(format!("Task {verb}{suffix}: {display}{detail}")).style(style)]
}

/// Render Grok's inline BTW side-question item.
pub fn render_btw(question: &str, answer: Option<&str>, status: &str, expanded: bool) -> Vec<Line<'static>> {
    let style = style_tool_summary();
    let marker = if status == "running" { "/btw…" } else { "/btw" };
    let mut lines = vec![Line::from(format!("{marker} {question}")).style(style)];
    if expanded {
        if let Some(answer) = answer.filter(|text| !text.is_empty()) {
            lines.push(Line::from(format!("  {answer}")).style(style));
        }
    }
    lines
}

/// Grok's feed names built-in tools by action, while preserving `Run` for
/// shell and unknown integrations. This belongs to feed presentation only;
/// protocol/tool names remain unchanged everywhere else.
fn feed_tool_label_parts(name: &str, args: &str) -> (String, String) {
    let action = match name {
        "read" | "read_file" => Some("Read"),
        "list_dir" | "list_directory" => Some("List"),
        "grep" | "find" | "search" | "search_files" => Some("Search"),
        "edit" | "edit_file" | "write_file" => Some("Edit"),
        "fetch" | "fetch_docs" | "web_fetch" => Some("Fetch"),
        "web_search" | "search_web" => Some("Web Search"),
        "memory_search" | "search_memory" => Some("Memory Search"),
        _ => None,
    };
    if let Some(action) = action {
        let trimmed = args.trim();
        if trimmed.is_empty() {
            (action.to_string(), String::new())
        } else {
            (action.to_string(), format!(" {}", trimmed.trim_matches('\"')))
        }
    } else {
        format_tool_label_parts(name, args)
    }
}

fn feed_tool_done_label_parts(name: &str, args: &str, output: &str) -> (String, String) {
    let (verb, args_part) = feed_tool_label_parts(name, args);
    if matches!(name, "list_dir" | "list_directory") {
        let count = output.lines().filter(|line| !line.trim().is_empty()).count();
        if count > 0 {
            let noun = if count == 1 { "entry" } else { "entries" };
            return (verb, format!("{args_part} ({count} {noun})"));
        }
    }
    (verb, args_part)
}

/// Render a swarm subagent lifecycle row (GROK.md §26).
///
/// Running:   `❙  ◆ Subagent running: “<desc>” — <activity> (<model>)`
/// Completed: `◆ Subagent completed in Xs: “<desc>”`
/// Failed:    `◆ Subagent failed in Xs: “<desc>”`
///
/// Expanded finished rows render the worker output indented under the row,
/// styled like an expanded thought body.
#[allow(clippy::too_many_lines)]
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
            let activity_text = if activity.is_empty() {
                "Running"
            } else {
                activity
            };
            // Pulse the rail/bar/diamond toward background using pulse_brightness (grok parity)
            let pulse = pulse_brightness(animation_frame, 0.08);
            let rail_color = blend_color(color_bg(), crate::theme::color_rail_running(), pulse)
                .unwrap_or_else(crate::theme::color_rail_running);
            let bar_color =
                blend_color(color_bg(), color_subagent_running_bar(), pulse).unwrap_or(color_subagent_running_bar());
            let diamond_color = blend_color(color_bg(), color_subagent_running_diamond(), pulse)
                .unwrap_or(color_subagent_running_diamond());
            let dim_color =
                blend_color(color_bg(), color_subagent_running_dim(), pulse).unwrap_or(color_subagent_running_dim());
            Line::from(vec![
                Span::styled(RAIL_GLYPH.to_string(), Style::new().fg(rail_color)),
                Span::styled(GLYPH_SUBAGENT_BAR, Style::new().fg(bar_color)),
                Span::styled(" ", Style::new().fg(bar_color)),
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
            Span::styled(
                RAIL_GLYPH.to_string(),
                Style::new().fg(crate::theme::color_rail_success()),
            ),
            Span::styled(
                GLYPH_SUBAGENT_DIAMOND,
                Style::new().fg(color_subagent_completed_diamond()),
            ),
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
            Span::styled(
                RAIL_GLYPH.to_string(),
                Style::new().fg(crate::theme::color_rail_error()),
            ),
            Span::styled(
                GLYPH_SUBAGENT_DIAMOND,
                Style::new().fg(color_subagent_failed_diamond()),
            ),
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
            Span::styled(
                RAIL_GLYPH.to_string(),
                Style::new().fg(crate::theme::color_rail_error()),
            ),
            Span::styled(
                GLYPH_SUBAGENT_DIAMOND,
                Style::new().fg(color_subagent_failed_diamond()),
            ),
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
    // Feed-level accent/bullet chrome is composed by the shared feed wrapper.
    // Strip the legacy inline chrome before returning the block content.
    for line in &mut lines {
        if let Some(index) = line
            .spans
            .iter()
            .position(|span| span.content.starts_with("Subagent "))
        {
            line.spans.drain(..index);
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
        runie_core::Element::ToolDone { name, .. } | runie_core::Element::ToolSummary { name, .. } => {
            Some(name.clone())
        }
        _ => None,
    }
}

fn render_context_tool(elem: &runie_core::Element) -> Vec<Line<'static>> {
    match elem {
        runie_core::Element::ToolDone { name, args, duration_secs, output, bytes_transferred, error, .. } => {
            render_tool_done(
                name,
                args,
                *duration_secs,
                output,
                *bytes_transferred,
                *error,
                &None,
                0,
            )
        }
        runie_core::Element::ToolSummary { name, duration_secs, .. } => render_tool_summary(name, "", *duration_secs),
        _ => Vec::new(),
    }
}

/// Render a blockquote from plain markdown text using tui_markdown.
/// `depth` controls how many `│` bars are stacked (1 for `> quote`, 2 for `>> nested`, etc.).
pub fn render_blockquote_from_spans(text: &str, base_color: Color, depth: usize) -> Vec<Line<'static>> {
    // Use tui_markdown for styling (via apply_color_to_inlines).
    let spans = apply_color_to_inlines(text, base_color);
    let mut lines = Vec::new();
    // Stack `depth` bars for nested quotes: 1 bar per level (grok parity).
    let bars = "│".repeat(depth);
    let prefix = format!("{} {} ", GLYPH_INDENT.trim_end(), bars);
    let prefix_width = str_width(&prefix) as u16;
    let content_width = 200u16; // Will be clamped by actual terminal width
    let rest_width = content_width.saturating_sub(prefix_width);

    let rows = wrap_styled_spans_for_blockquote(&spans, rest_width);
    for (i, row) in rows.iter().enumerate() {
        let line_prefix = if i == 0 { prefix.as_str() } else { "       " };
        // Blockquote bar is dim-colored.
        let dim_style = style_agent().dim();
        let mut line_spans = vec![Span::styled(line_prefix.to_owned(), dim_style)];
        line_spans.extend(md_to_spans(row));
        lines.push(Line::from(line_spans).style(dim_style));
    }
    if lines.is_empty() {
        lines.push(Line::from(format!("{} {} ", GLYPH_INDENT.trim_end(), bars)).style(style_agent().dim()));
    }
    lines
}

/// Render a horizontal rule as a line of box-drawing dashes (U+2500).
pub fn render_horizontal_rule(content_width: u16, ts_str: &str, is_first: bool) -> Vec<Line<'static>> {
    let rule_char = "─"; // U+2500 BOX DRAWINGS LIGHT HORIZONTAL
    let ts_width: u16 = (str_width(ts_str) + 1) as u16;
    let rule_len = if is_first {
        content_width.saturating_sub(ts_width) as usize
    } else {
        content_width as usize
    };
    let rule = rule_char.repeat(rule_len.max(3));

    let mut line_spans: Vec<Span<'static>> = Vec::new();

    if is_first {
        // Add padding before rule to align with content.
        let text_width = str_width(&rule) as u16;
        let padding = content_width.saturating_sub(text_width).saturating_sub(ts_width);
        if padding > 0 {
            line_spans.push(Span::raw(" ".repeat(padding as usize)));
        }
        line_spans.push(Span::styled(rule, style_agent().dim()));
        line_spans.push(Span::styled(format!(" {}", ts_str), style_feed_timestamp()));
    } else {
        line_spans.push(Span::styled(rule, style_agent().dim()));
    }

    vec![Line::from(line_spans)]
}

/// Wrap styled spans for blockquote rendering.
///
/// Uses `textwrap` for display-width-aware wrapping. Each original span is kept
/// intact where possible; long spans are broken character-by-character (preserving
/// style). The result is a list of rows, each row being a list of spans that fit
/// within `max_width`.
#[allow(clippy::assigning_clones, clippy::redundant_clone)]
#[allow(clippy::too_many_lines)]
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
                current_row.push(MdSpan { content: line_owned, style: span.style });
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

// ---------------------------------------------------------------------------
// Special content type renderers (images, tables, diffs, tool confirmations)
// ---------------------------------------------------------------------------

use runie_core::view::elements::{DiffType, ImageProtocol, WebSearchResult};

/// Render an inline image element.
///
/// For iTerm2/Kitty protocols, outputs the terminal escape sequence prefix.
/// Actual image rendering happens via terminal control sequences written directly
/// to the terminal buffer. We output a placeholder showing dimensions.
pub fn render_image(
    _data: &str,
    mime_type: &str,
    width_cells: Option<u16>,
    height_cells: Option<u16>,
    protocol: ImageProtocol,
    _timestamp: f64,
) -> Vec<Line<'static>> {
    use crate::theme::{style_agent, GLYPH_INDENT};

    let style = style_agent();
    let dim_str = match (width_cells, height_cells) {
        (Some(w), Some(h)) => format!("{}x{}", w, h),
        (Some(w), None) => format!("{} cells wide", w),
        _ => "auto".to_string(),
    };
    let protocol_str = match protocol {
        ImageProtocol::ITerm2 => "iTerm2",
        ImageProtocol::Kitty => "Kitty",
        ImageProtocol::Sixel => "Sixel",
    };
    let mime_owned = mime_type.to_string();

    vec![
        Line::from(vec![
            Span::styled(GLYPH_INDENT, style),
            Span::styled("[Image: ", style),
            Span::styled(mime_owned, style.bold()),
            Span::styled(format!(" | {} | {}]", dim_str, protocol_str), style),
        ]),
        Line::from(vec![
            Span::styled(GLYPH_INDENT, style),
            Span::styled("  └─ ", style),
            Span::styled("Rendered via terminal graphics protocol", style),
        ]),
    ]
}

/// Render a structured data/JSON part with optional format string.
pub fn render_data_part(data: &str, format_string: Option<&str>, _timestamp: f64) -> Vec<Line<'static>> {
    use crate::theme::{style_agent, style_tool_output, GLYPH_INDENT};

    let style = style_agent();
    let output_style = style_tool_output();
    let label = format_string.unwrap_or("data");
    let display_data = if data.len() > 200 {
        format!("{}...", &data[..200])
    } else {
        data.to_string()
    };

    vec![
        Line::from(vec![
            Span::styled(GLYPH_INDENT, style),
            Span::styled(format!("[{}]", label), style.bold()),
        ]),
        Line::from(vec![
            Span::styled(format!("  {} ", GLYPH_INDENT), style),
            Span::styled(display_data, output_style),
        ]),
    ]
}

/// Render a markdown table with headers, rows, and column alignments using box-drawing characters.
#[allow(clippy::cognitive_complexity)]
#[allow(clippy::too_many_lines)]
pub fn render_markdown_table(
    headers: &[String],
    rows: &[Vec<String>],
    alignments: &[Option<bool>],
    _timestamp: f64,
) -> Vec<Line<'static>> {
    use crate::theme::{style_agent, GLYPH_INDENT};

    let style = style_agent();
    let mut lines = Vec::new();

    if headers.is_empty() {
        return lines;
    }

    // Calculate column widths
    let col_count = headers.len();
    let mut col_widths: Vec<usize> = headers.iter().map(|h| h.len()).collect();
    for row in rows {
        for (i, cell) in row.iter().enumerate() {
            if i < col_count {
                col_widths[i] = col_widths[i].max(cell.len());
            }
        }
    }

    // Build box-drawing borders
    let dash = "─"; // U+2500 BOX DRAWINGS LIGHT HORORIZONTAL
    let tee_down = "┬"; // U+252C
    let tee_up = "┴"; // U+2534
    let cross = "┼"; // U+253C
    let left_tee = "├"; // U+251C
    let right_tee = "┤"; // U+2524

    // Top border: ┌─┬─┐
    let mut top_border = format!("{}┌", GLYPH_INDENT);
    for (i, width) in col_widths.iter().enumerate() {
        top_border.push_str(&dash.repeat(*width));
        if i < col_widths.len() - 1 {
            top_border.push_str(tee_down);
        }
    }
    top_border.push('┐');
    lines.push(Line::from(top_border).style(style));

    // Header row with vertical bars
    let mut header_spans = vec![Span::styled(format!("{}│", GLYPH_INDENT), style)];
    for (i, header) in headers.iter().enumerate() {
        let width = col_widths[i];
        let aligned = if i < alignments.len() {
            alignments[i]
        } else {
            None
        };
        let cell_str = match aligned {
            Some(true) => format!("{:>width$}", header, width = width), // right
            Some(false) => format!("{:^width$}", header, width = width), // center
            None => format!("{:<width$}", header, width = width),       // left
        };
        header_spans.push(Span::styled(format!(" {} │", cell_str), style.bold().underlined()));
    }
    lines.push(Line::from(header_spans));

    // Header separator: ├─┼─┤
    let mut header_sep = format!("{}├", GLYPH_INDENT);
    for (i, width) in col_widths.iter().enumerate() {
        header_sep.push_str(&dash.repeat(*width));
        if i < col_widths.len() - 1 {
            header_sep.push_str(cross);
        }
    }
    header_sep.push('┤');
    lines.push(Line::from(header_sep).style(style));

    // Render data rows
    for row in rows {
        let mut row_spans = vec![Span::styled(format!("{}│", GLYPH_INDENT), style)];
        for (i, cell) in row.iter().enumerate() {
            if i >= col_count {
                break;
            }
            let width = col_widths[i];
            let aligned = if i < alignments.len() {
                alignments[i]
            } else {
                None
            };
            let cell_str = match aligned {
                Some(true) => format!("{:>width$}", cell, width = width),
                Some(false) => format!("{:^width$}", cell, width = width),
                None => format!("{:<width$}", cell, width = width),
            };
            row_spans.push(Span::styled(format!(" {} │", cell_str), style));
        }
        lines.push(Line::from(row_spans));
    }

    // Bottom border: └─┴─┘
    let mut bottom_border = format!("{}└", GLYPH_INDENT);
    for (i, width) in col_widths.iter().enumerate() {
        bottom_border.push_str(&dash.repeat(*width));
        if i < col_widths.len() - 1 {
            bottom_border.push_str(tee_up);
        }
    }
    bottom_border.push('┘');
    lines.push(Line::from(bottom_border).style(style));

    lines
}

/// Render diff/changelist output with type indicator.
#[allow(clippy::too_many_lines)]
pub fn render_diff_output(content: &str, diff_type: DiffType, _timestamp: f64) -> Vec<Line<'static>> {
    use crate::theme::{style_agent, style_tool_output, GLYPH_INDENT};

    let style = style_agent();
    let output_style = style_tool_output();
    let type_str = match diff_type {
        DiffType::Unified => "unified",
        DiffType::SideBySide => "side-by-side",
        DiffType::Context => "context",
    };

    let header = Line::from(vec![
        Span::styled(GLYPH_INDENT, style),
        Span::styled("[Diff: ", style),
        Span::styled(type_str, style.bold()),
        Span::styled("]", style),
    ]);

    let mut lines = vec![header];
    for line in content.lines().take(50) {
        let line_owned = line.to_string();
        let diff_line = if line.starts_with("+++") || line.starts_with("---") {
            Line::from(vec![
                Span::styled(format!("{} ", GLYPH_INDENT), style),
                Span::styled(line_owned.clone(), style),
            ])
        } else if let Some(stripped) = line.strip_prefix('+') {
            Line::from(vec![
                Span::styled(format!("{} +", GLYPH_INDENT), style),
                Span::styled(stripped.to_string(), style.green()),
            ])
        } else if let Some(stripped) = line.strip_prefix('-') {
            Line::from(vec![
                Span::styled(format!("{} -", GLYPH_INDENT), style),
                Span::styled(stripped.to_string(), style.red()),
            ])
        } else if line.starts_with("@@") {
            Line::from(vec![
                Span::styled(format!("{} ", GLYPH_INDENT), style),
                Span::styled(line_owned.clone(), style.cyan()),
            ])
        } else {
            Line::from(vec![
                Span::styled(format!("{} ", GLYPH_INDENT), style),
                Span::styled(line_owned, output_style),
            ])
        };
        lines.push(diff_line);
    }

    if content.lines().count() > 50 {
        lines.push(Line::from(vec![
            Span::styled(format!("{} ", GLYPH_INDENT), style),
            Span::styled("... (truncated)", style),
        ]));
    }

    lines
}

/// Render an Anthropic-style thinking block.
#[allow(clippy::too_many_lines)]
pub fn render_anthropic_thinking(
    content: &str,
    signature: Option<String>,
    redacted: bool,
    _timestamp: f64,
) -> Vec<Line<'static>> {
    use crate::theme::{style_agent, style_thinking, GLYPH_INDENT};

    let base_style = if redacted {
        style_thinking()
    } else {
        style_agent()
    };

    let mut lines = Vec::new();

    // Header
    let header_text = if redacted {
        "[Redacted Thinking]"
    } else {
        "[Thinking]"
    };
    lines.push(Line::from(vec![
        Span::styled(GLYPH_INDENT, base_style),
        Span::styled(header_text, base_style.bold()),
    ]));

    // Signature if present
    if let Some(sig) = &signature {
        let tail = if sig.len() >= 8 {
            format!("...{}", &sig[sig.len() - 8..])
        } else {
            sig.clone()
        };
        lines.push(Line::from(vec![
            Span::styled(format!("{}   sig: ", GLYPH_INDENT), base_style),
            Span::styled(tail, base_style),
        ]));
    }

    // Content (if not redacted)
    if !redacted {
        for raw_line in content.lines() {
            if raw_line.is_empty() {
                lines.push(Line::from("").style(base_style));
            } else {
                for chunk in word_wrap(raw_line, 80, 80) {
                    lines.push(Line::from(vec![
                        Span::styled(format!("{}   ", GLYPH_INDENT), base_style),
                        Span::styled(chunk.to_string(), base_style),
                    ]));
                }
            }
        }
    } else {
        lines.push(Line::from(vec![Span::styled(
            format!("{}   [encrypted content]", GLYPH_INDENT),
            base_style,
        )]));
    }

    lines
}

/// Render a web search call with results.
pub fn render_web_search_call(query: &str, results: &[WebSearchResult], _timestamp: f64) -> Vec<Line<'static>> {
    use crate::theme::{style_agent, GLYPH_INDENT};

    let style = style_agent();
    let mut lines = Vec::new();

    let site_count = results
        .iter()
        .filter_map(|result| web_search_domain(&result.url))
        .collect::<std::collections::HashSet<_>>()
        .len();
    let suffix = match site_count {
        0 => String::new(),
        1 => " (1 site)".to_string(),
        count => format!(" ({count} sites)"),
    };

    // Grok-style search header includes a deduplicated citation-domain summary.
    lines.push(Line::from(vec![
        Span::styled(GLYPH_INDENT, style),
        Span::styled("Web Search ", style.bold()),
        Span::styled(query.to_string(), style),
        Span::styled(suffix, style.dim()),
    ]));

    // Results
    for (i, result) in results.iter().enumerate().take(5) {
        let title_owned = result.title.clone();
        let snippet_owned = result.snippet.clone();
        let url_owned = result.url.clone();
        lines.push(Line::from(vec![
            Span::styled(format!("{} ", GLYPH_INDENT), style),
            Span::styled(format!("{}. ", i + 1), style.bold()),
            Span::styled(title_owned, style.underlined()),
        ]));
        lines.push(Line::from(vec![
            Span::styled(format!("{}   ", GLYPH_INDENT), style),
            Span::styled(snippet_owned, style),
        ]));
        lines.push(Line::from(vec![
            Span::styled(format!("{}   ", GLYPH_INDENT), style),
            Span::styled(url_owned, style.dim()),
        ]));
    }

    if results.is_empty() {
        lines.push(Line::from(vec![
            Span::styled(format!("{}   ", GLYPH_INDENT), style),
            Span::styled("Searching...", style),
        ]));
    }

    lines
}

fn web_search_domain(url: &str) -> Option<String> {
    let host = url
        .split_once("://")
        .map(|(_, rest)| rest)
        .unwrap_or(url)
        .split(['/', '?', '#'])
        .next()
        .unwrap_or_default()
        .trim_start_matches("www.")
        .to_ascii_lowercase();
    (!host.is_empty() && host.contains('.')).then_some(host)
}

/// Render ANSI escape sequence styled content.
pub fn render_ansi_styled(raw_content: &str, _plain_text: &str, _timestamp: f64) -> Vec<Line<'static>> {
    use crate::theme::{style_agent, style_tool_output, GLYPH_INDENT};

    let style = style_agent();
    let output_style = style_tool_output();
    let mut lines = Vec::new();

    // Header
    lines.push(Line::from(vec![
        Span::styled(GLYPH_INDENT, style),
        Span::styled("[ANSI Styled]", style.bold()),
    ]));

    // Render the ANSI content - show the colored version with fallback
    for line in raw_content.lines().take(20) {
        let spans = ansi_to_spans(line, output_style);
        if !spans.is_empty() {
            lines.push(Line::from(spans));
        } else {
            lines.push(Line::from(line.to_string()).style(output_style));
        }
    }

    if raw_content.lines().count() > 20 {
        lines.push(Line::from(vec![
            Span::styled(format!("{}   ", GLYPH_INDENT), style),
            Span::styled("... (truncated)", style),
        ]));
    }

    lines
}

/// Convert ANSI escape sequences to ratatui spans.
#[allow(clippy::cognitive_complexity)]
#[allow(clippy::too_many_lines)]
fn ansi_to_spans(input: &str, default_style: ratatui::style::Style) -> Vec<Span<'static>> {
    use ratatui::style::Color;

    let mut spans = Vec::new();
    let mut current_style = default_style;
    let mut current_text = String::new();
    let mut chars = input.chars().peekable();

    while let Some(ch) = chars.next() {
        if ch == '\x1B' {
            // Start of escape sequence
            let mut seq = String::from(ch);

            // Collect the escape sequence
            while let Some(&next) = chars.peek() {
                seq.push(next);
                chars.next();
                if next.is_ascii_alphabetic() || next == 'm' {
                    break;
                }
            }

            // Flush current text
            if !current_text.is_empty() {
                spans.push(Span::styled(
                    std::mem::take(&mut current_text),
                    current_style,
                ));
            }

            // Parse SGR (Select Graphic Rendition) parameters
            if seq.ends_with('m') {
                let params = &seq[2..seq.len() - 1];
                if params.is_empty() || params == "0" {
                    current_style = default_style;
                } else {
                    for part in params.split(';') {
                        match part.parse::<u8>().unwrap_or(0) {
                            1 => current_style = current_style.bold(),
                            2 => current_style = current_style.dim(),
                            3 => current_style = current_style.italic(),
                            4 => current_style = current_style.underlined(),
                            30 => current_style = current_style.fg(Color::Black),
                            31 => current_style = current_style.fg(Color::Red),
                            32 => current_style = current_style.fg(Color::Green),
                            33 => current_style = current_style.fg(Color::Yellow),
                            34 => current_style = current_style.fg(Color::Blue),
                            35 => current_style = current_style.fg(Color::Magenta),
                            36 => current_style = current_style.fg(Color::Cyan),
                            37 => current_style = current_style.fg(Color::White),
                            90..=97 => {
                                current_style = current_style.fg(Color::Indexed(part.parse::<u8>().unwrap_or(90) - 90))
                            }
                            _ => {}
                        }
                    }
                }
            }
        } else {
            current_text.push(ch);
        }
    }

    // Flush remaining text
    if !current_text.is_empty() {
        spans.push(Span::styled(current_text, current_style));
    }

    if spans.is_empty() && !input.is_empty() {
        spans.push(Span::styled(input.to_string(), default_style));
    }

    spans
}

/// Render a tool confirmation request.
#[allow(clippy::too_many_lines)]
pub fn render_tool_confirmation(
    request_id: &str,
    name: &str,
    args: &str,
    description: &str,
    _timestamp: f64,
) -> Vec<Line<'static>> {
    use crate::theme::{style_agent, GLYPH_INDENT, GLYPH_X};

    let style = style_agent();
    let name_owned = name.to_string();
    let desc_owned = description.to_string();
    let args_owned = args.to_string();
    let request_id_owned = request_id.to_string();
    let mut lines = Vec::new();

    // Header with warning style
    lines.push(Line::from(vec![
        Span::styled(GLYPH_INDENT, style),
        Span::styled(format!("{} ", GLYPH_X), style.red()),
        Span::styled("[CONFIRM]", style.bold().red()),
        Span::styled(" Tool call requires approval", style),
    ]));

    // Tool name and description
    lines.push(Line::from(vec![
        Span::styled(format!("{}   ", GLYPH_INDENT), style),
        Span::styled("Tool: ", style.bold()),
        Span::styled(name_owned.clone(), style),
    ]));

    if !description.is_empty() {
        lines.push(Line::from(vec![
            Span::styled(format!("{}   ", GLYPH_INDENT), style),
            Span::styled("Action: ", style.bold()),
            Span::styled(desc_owned.clone(), style),
        ]));
    }

    // Arguments
    if !args.is_empty() {
        lines.push(Line::from(vec![
            Span::styled(format!("{}   ", GLYPH_INDENT), style),
            Span::styled("Args: ", style.bold()),
        ]));
        for line in args_owned.lines().take(5) {
            let line_owned = line.to_string();
            lines.push(Line::from(vec![
                Span::styled(format!("{}     ", GLYPH_INDENT), style),
                Span::styled(line_owned, style),
            ]));
        }
    }

    // Request ID (for debugging/reference)
    lines.push(Line::from(vec![
        Span::styled(format!("{}   ", GLYPH_INDENT), style),
        Span::styled("Request ID: ", style.dim()),
        Span::styled(request_id_owned, style.dim()),
    ]));

    // Action hint
    lines.push(Line::from(vec![
        Span::styled(format!("{} ", GLYPH_INDENT), style),
        Span::styled("Press ", style),
        Span::styled("y", style.bold()),
        Span::styled(" to confirm, ", style),
        Span::styled("n", style.bold()),
        Span::styled(" to deny", style),
    ]));

    lines
}
