//! View — renders Snapshot to terminal via ratatui
//!
//! Architecture: the event loop builds immutable Snapshots;
//! the render actor draws them. No state mutations, no blocking
//! I/O, no caching — pure functions from Snapshot to Frame.
//!
//! DESIGN SYSTEM RULE: all colors, glyphs, and styles come from
//! crate::theme only. No literals, no hardcoded values.

use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::Style,
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState, Wrap},
    Frame,
};

use runie_core::{Element, Snapshot};

use crate::markdown::{extract_code_blocks, md_to_spans, parse_inline_markdown, parse_inline_markdown_with_color, CodeBlock};
use runie_core::format_timestamp;
use crate::theme::{
    C, GLYPH_USER, GLYPH_AGENT, GLYPH_TOOL, GLYPH_INDENT,
    GLYPH_SELECTED, GLYPH_UNSELECTED, PANEL_CHAT, PANEL_INPUT,
    SCROLLBAR_TRACK, SCROLLBAR_THUMB, INDICATOR_COLLAPSED,
    code_header_label, thinking_line, tool_running_line, tool_done_header,
    tool_summary_line, turn_complete_line, thought_summary_line,
    style_user, style_agent, style_thought, style_thinking, style_thought_summary,
    style_tool_running, style_tool_header, style_tool_output, style_tool_summary,
    style_turn_complete, style_empty_state, style_timestamp, style_status_idle,
    style_status_active, style_border, style_border_flash, style_code_block,
    style_code_header, style_input_cursor, style_placeholder, style_hint,
    style_popup_selected, style_popup_unselected, style_popup_border,
};

/// Draw a Snapshot to the terminal. Pure function — no mutable state.
pub fn draw_snapshot(f: &mut Frame, snap: &Snapshot) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(3),
            Constraint::Length(1),
            Constraint::Length(3),
            Constraint::Length(1),
        ])
        .split(f.area());

    messages(f, snap, chunks[0]);
    status(f, snap, chunks[1]);
    input(f, snap, chunks[2]);
    hints(f, snap, chunks[3]);
    at_suggestions(f, snap);
}

/// Legacy entry point for code that still builds AppState directly.
pub fn view(f: &mut Frame, state: &mut runie_core::AppState) {
    state.ensure_fresh();
    let snap = state.snapshot();
    draw_snapshot(f, &snap);
}

fn status(f: &mut Frame, snap: &Snapshot, area: Rect) {
    let tokens: usize = snap.elements.iter().map(|e| estimate_element_tokens(e)).sum();
    let left_text = build_status_text(snap);
    let right_text = format!("{} tok", tokens);

    let hchunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Min(0), Constraint::Length(right_text.len() as u16)])
        .split(area);

    let status_style = if snap.turn_active {
        style_status_active()
    } else {
        style_status_idle()
    };

    f.render_widget(Paragraph::new(left_text).style(status_style), hchunks[0]);
    f.render_widget(
        Paragraph::new(right_text).style(style_timestamp()),
        hchunks[1],
    );
}

fn build_status_text(snap: &Snapshot) -> String {
    let mut parts = Vec::new();
    parts.push(format!("{}/{}", snap.provider, snap.model));
    if snap.turn_active {
        if let Some(elapsed) = snap.turn_elapsed_secs {
            parts.push(runie_core::labels::action_text(
                snap.spinner_frame, "Working", elapsed,
            ));
        } else {
            parts.push(format!("{} Working...", snap.spinner_frame));
        }
    }
    parts.join(" · ")
}

fn messages(f: &mut Frame, snap: &Snapshot, area: Rect) {
    if snap.elements.is_empty() {
        render_empty_state(f, area);
        return;
    }
    render_message_content(f, snap, area);
}

fn render_empty_state(f: &mut Frame, area: Rect) {
    let hint = Line::from("Type a message to start...").style(style_empty_state());
    f.render_widget(Paragraph::new(hint), area);
}

fn render_message_content(f: &mut Frame, snap: &Snapshot, area: Rect) {
    let height = area.height as usize;
    let total_lines = snap.total_lines;
    if height == 0 || total_lines == 0 {
        return;
    }

    let show_bar = total_lines > height;
    let content_width = if show_bar {
        area.width.saturating_sub(1)
    } else {
        area.width
    };

    let hchunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Length(content_width), Constraint::Min(0)])
        .split(area);

    let lines = build_lines(snap);
    let offset = snap.scroll_offset(height);
    f.render_widget(
        Paragraph::new(lines)
            .scroll((offset, 0))
            .wrap(Wrap { trim: false }),
        hchunks[0],
    );

    if show_bar {
        render_scrollbar(f, area, total_lines, offset, height);
    }
}

fn build_lines(snap: &Snapshot) -> Vec<Line<'_>> {
    let mut lines = Vec::with_capacity(snap.total_lines);
    for elem in &snap.elements {
        lines.extend(to_lines(elem, snap.spinner_frame));
    }
    lines
}

fn render_scrollbar(f: &mut Frame, area: Rect, total: usize, offset: u16, height: usize) {
    let scrollbar = Scrollbar::default()
        .orientation(ScrollbarOrientation::VerticalRight)
        .begin_symbol(None)
        .end_symbol(None)
        .track_symbol(Some(SCROLLBAR_TRACK))
        .thumb_symbol(SCROLLBAR_THUMB);

    let mut state = ScrollbarState::new(total)
        .position(offset as usize)
        .viewport_content_length(height);
    f.render_stateful_widget(scrollbar, area, &mut state);
}

fn to_lines<'a>(elem: &'a Element, _spinner_frame: char) -> Vec<Line<'a>> {
    use runie_core::Element::*;
    match elem {
        Spacer { .. } => vec![Line::from("")],
        UserMessage { content, timestamp } => render_user_message(content, *timestamp),
        AgentMessage { content, timestamp } => render_agent_message(content, *timestamp),
        Thinking { started, .. } => vec![Line::from(
            thinking_line(started.elapsed().as_secs_f64())
        ).style(style_thinking())],
        ThoughtSummary { content, .. } => vec![Line::from(
            thought_summary_line(content.lines().next().unwrap_or(content))
        ).style(style_thought_summary())],
        ThoughtMarker { content, .. } => render_thought_marker(content),
        ToolRunning { name, started, .. } => vec![Line::from(
            tool_running_line(name, started.elapsed().as_secs_f64())
        ).style(style_tool_running())],
        ToolDone { name, duration_secs, output, .. } => render_tool_done(name, *duration_secs, output),
        ToolSummary { name, duration_secs, .. } => vec![Line::from(
            tool_summary_line(name, *duration_secs)
        ).style(style_tool_summary())],
        TurnComplete { duration_secs, .. } => vec![Line::from(
            turn_complete_line(*duration_secs)
        ).style(style_turn_complete())],
    }
}

fn render_user_message(content: &str, timestamp: f64) -> Vec<Line<'static>> {
    let mut lines = Vec::new();
    let ts_str = format_timestamp(timestamp);
    for (i, line) in content.lines().enumerate() {
        let ts = format!(" {:>5}", ts_str);
        let prefix = if i == 0 { GLYPH_USER } else { GLYPH_INDENT };
        let mut spans = vec![Span::styled(prefix, style_user())];
        spans.extend(md_to_spans(&parse_inline_markdown_with_color(line, C.fg_bright)));
        spans.push(Span::styled(ts, style_timestamp()));
        lines.push(Line::from(spans));
    }
    if lines.is_empty() {
        lines.push(Line::from(format!("{}{:>5}", GLYPH_USER, ts_str)).style(style_user()));
    }
    lines
}

fn render_agent_message(content: &str, timestamp: f64) -> Vec<Line<'static>> {
    let blocks = extract_code_blocks(content);
    let mut lines = Vec::new();
    let mut is_first = true;

    let ts_str = format_timestamp(timestamp);
    for block in blocks {
        match block {
            CodeBlock::Text(text) => {
                for line in text.lines() {
                    lines.push(render_agent_markdown_line(
                        line, timestamp, is_first
                    ));
                    is_first = false;
                }
            }
            CodeBlock::Code { lang, content } => {
                lines.push(render_code_header(&lang, is_first));
                is_first = false;
                for line in content.lines() {
                    let text = format!("{}{} {:>5}", GLYPH_INDENT, line, ts_str);
                    lines.push(Line::from(text).style(style_code_block()));
                }
            }
        }
    }
    if lines.is_empty() {
        lines.push(Line::from(format!("{}{:>5}", GLYPH_AGENT, ts_str)).style(style_agent()));
    }
    lines
}

fn render_code_header(lang: &str, is_first: bool) -> Line<'static> {
    let prefix = if is_first { GLYPH_AGENT } else { GLYPH_INDENT };
    Line::from(code_header_label(prefix, lang)).style(style_code_header())
}

fn render_agent_markdown_line(
    line: &str,
    timestamp: f64,
    is_first: bool,
) -> Line<'static> {
    let prefix = if is_first { GLYPH_AGENT } else { GLYPH_INDENT };
    let ts = format!(" {:>5}", format_timestamp(timestamp));
    let mut spans = vec![Span::styled(prefix.to_string(), style_agent())];
    spans.extend(md_to_spans(&parse_inline_markdown_with_color(line, C.fg)));
    spans.push(Span::styled(ts, style_timestamp()));
    Line::from(spans)
}

fn render_markdown_line(
    line: &str,
    timestamp: f64,
    is_first: bool,
    first_prefix: &str,
    rest_prefix: &str,
    base_style: Style,
) -> Line<'static> {
    let prefix = if is_first { first_prefix } else { rest_prefix };
    let ts = format!(" {:>5}", format_timestamp(timestamp));
    let mut spans = vec![Span::styled(prefix.to_string(), base_style)];
    spans.extend(md_to_spans(&parse_inline_markdown(line)));
    spans.push(Span::styled(ts, style_timestamp()));
    Line::from(spans)
}

fn render_thought_marker(content: &str) -> Vec<Line<'static>> {
    content.lines()
        .map(|line| Line::from(line.to_string()).style(style_thought()))
        .collect()
}

fn render_tool_done(name: &str, duration_secs: f64, output: &str) -> Vec<Line<'static>> {
    let mut lines = vec![Line::from(tool_done_header(name, duration_secs))
        .style(style_tool_header())];
    if !output.is_empty() {
        for line in output.lines() {
            lines.push(Line::from(line.to_string()).style(style_tool_output()));
        }
    }
    lines
}

fn input(f: &mut Frame, snap: &Snapshot, area: Rect) {
    let border_style = if snap.input_flash > 0 {
        style_border_flash()
    } else {
        style_border()
    };
    let block = Block::default()
        .borders(Borders::ALL)
        .title(PANEL_INPUT)
        .border_style(border_style);
    let inner = block.inner(area);

    let spans = build_input_spans(snap);
    f.render_widget(Paragraph::new(Line::from(spans)).block(block), area);

    let cursor_x = inner.x + (GLYPH_USER.len() + snap.cursor_pos.min(snap.input.len())) as u16;
    f.set_cursor_position((cursor_x, inner.y));
}

fn build_input_spans(snap: &Snapshot) -> Vec<Span<'_>> {
    if snap.input.is_empty() && !snap.placeholder.is_empty() {
        return vec![
            Span::styled(GLYPH_USER, style_user()),
            Span::styled(snap.placeholder.clone(), style_placeholder()),
        ];
    }
    let cursor_pos = snap.cursor_pos.min(snap.input.len());
    let before = &snap.input[..cursor_pos];
    let (at_cursor, after) = if cursor_pos < snap.input.len() {
        let c = snap.input[cursor_pos..].chars().next().unwrap();
        let char_len = c.len_utf8();
        (c, &snap.input[cursor_pos + char_len..])
    } else {
        (' ', "")
    };
    vec![
        Span::styled(GLYPH_USER, style_user()),
        Span::styled(before, style_user()),
        Span::styled(at_cursor.to_string(), style_input_cursor()),
        Span::styled(after, style_user()),
    ]
}

fn hints(f: &mut Frame, _snap: &Snapshot, area: Rect) {
    let hints_text = "Ctrl+Shift+E=expand/collapse | Alt+Enter=follow-up | Esc=clear | Ctrl+C=quit";
    f.render_widget(Paragraph::new(hints_text).style(style_hint()), area);
}

fn at_suggestions(f: &mut Frame, snap: &Snapshot) {
    let suggestions = match &snap.at_suggestions {
        Some(s) if !s.is_empty() => s,
        _ => return,
    };
    let selected = snap.at_selected.unwrap_or(0).min(suggestions.len().saturating_sub(1));
    let area = f.area();
    let display_count = suggestions.len().min(8) as u16;
    let max_height = display_count + 4;
    let popup_area = Rect {
        x: area.x + 1,
        y: area.y + area.height.saturating_sub(4 + max_height),
        width: area.width.saturating_sub(2).max(20),
        height: max_height,
    };
    let mut lines: Vec<Line> = suggestions
        .iter()
        .take(8)
        .enumerate()
        .map(|(i, s)| {
            let prefix = if i == selected { GLYPH_SELECTED } else { GLYPH_UNSELECTED };
            let style = if i == selected { style_popup_selected() } else { style_popup_unselected() };
            Line::from(format!("{}{}", prefix, s)).style(style)
        })
        .collect();
    lines.push(Line::from(""));
    lines.push(
        Line::from("Tab=cycle Enter=insert Esc=close").style(style_hint()),
    );
    let block = Block::default()
        .borders(Borders::ALL)
        .title(format!(" @ files ({}) ", suggestions.len()))
        .border_style(style_popup_border());
    f.render_widget(Paragraph::new(lines).block(block), popup_area);
}

fn estimate_element_tokens(elem: &Element) -> usize {
    use runie_core::Element::*;
    match elem {
        UserMessage { content, .. }
        | AgentMessage { content, .. }
        | ThoughtMarker { content, .. } => content.len() / 4,
        Thinking { .. }
        | ThoughtSummary { .. }
        | ToolSummary { .. }
        | TurnComplete { .. } => 10,
        ToolRunning { .. } => 10,
        ToolDone { output, .. } => output.len() / 4 + 10,
        Spacer { .. } => 0,
    }
}
