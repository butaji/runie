//! View — renders Snapshot to terminal via ratatui
//!
//! Architecture: the event loop builds immutable Snapshots;
//! the render actor draws them. No state mutations, no blocking
//! I/O, no caching — pure functions from Snapshot to Frame.
//!
//! DESIGN SYSTEM RULE: all colors, glyphs, and styles come from
//! crate::theme only. No literals, no hardcoded values.

use ratatui::{
    layout::{Constraint, Direction, Layout, Margin, Rect},
    style::Style,
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState, Wrap},
    Frame,
};

use runie_core::{Element, Snapshot};

use crate::message as msg;
use crate::theme::{
    SCROLLBAR_TRACK, SCROLLBAR_THUMB,
    style_empty_state, style_timestamp, style_status_idle, style_status_active,
    style_input_cursor, style_placeholder, style_hint, style_hint_key,
    style_scrollbar, color_bg, set_current_theme, block_input, style_chevron,
};

fn vstack(area: Rect, heights: &[Constraint]) -> Vec<Rect> {
    Layout::default().direction(Direction::Vertical).constraints(heights).split(area).to_vec()
}

fn hstack(area: Rect, widths: &[Constraint]) -> Vec<Rect> {
    Layout::default().direction(Direction::Horizontal).constraints(widths).split(area).to_vec()
}

/// Draw a Snapshot to the terminal. Pure function — no mutable state.
pub fn draw_snapshot(f: &mut Frame, snap: &Snapshot) {
    set_current_theme(&snap.theme_name);
    let full_area = f.area();
    f.buffer_mut().set_style(full_area, Style::default().bg(color_bg()));
    let margin = if full_area.width > 20 && full_area.height > 10 {
        Margin::new(1, 1)
    } else {
        Margin::new(0, 0)
    };
    let area = full_area.inner(margin);
    let input_lines = count_input_lines(&snap.input);
    let input_height = (input_lines + 2).min(10) as u16;
    let c = vstack(area, &[
        Constraint::Min(3),
        Constraint::Length(1),
        Constraint::Length(input_height),
        Constraint::Length(1),
        Constraint::Length(1),
    ]);
    messages(f, snap, c[0]);
    status(f, snap, c[1]);
    input(f, snap, c[2]);
    hints(f, snap, c[4]);
    crate::popups::at_suggestions(f, snap);
    crate::popups::path_suggestions(f, snap);
    crate::popups::command_palette(f, snap);
    crate::popups::model_selector_dialog(f, snap);
    crate::popups::scoped_models_dialog(f, snap);
    crate::popups::settings_dialog(f, snap);
    crate::popups::session_tree_dialog(f, snap);
    crate::popups::panel::panel_dialog(f, snap);
}

/// Legacy entry point for code that still builds AppState directly.
pub fn view(f: &mut Frame, state: &mut runie_core::AppState) {
    state.ensure_fresh();
    let snap = state.snapshot();
    draw_snapshot(f, &snap);
}

fn status(f: &mut Frame, snap: &Snapshot, area: Rect) {
    let tokens: usize = snap.elements.iter().map(estimate_element_tokens).sum();
    let left_text = build_status_text(snap);
    let queue_part = if snap.queue_count > 0 {
        format!(" · {} queued", snap.queue_count)
    } else {
        String::new()
    };
    let right_text = format!("{} tok{}", tokens, queue_part);

    let h = hstack(area, &[
        Constraint::Min(0),
        Constraint::Length(right_text.len() as u16),
    ]);

    let status_style = if snap.turn_active { style_status_active() } else { style_status_idle() };
    f.render_widget(Paragraph::new(left_text).style(status_style), h[0]);
    f.render_widget(Paragraph::new(right_text).style(style_timestamp()), h[1]);
}

fn build_status_text(snap: &Snapshot) -> String {
    let mut parts = Vec::new();
    if snap.turn_active {
        if let Some(elapsed) = snap.turn_elapsed_secs {
            parts.push(runie_core::labels::action_text(
                snap.spinner_frame, "Working", elapsed,
            ));
        } else {
            parts.push(format!("{} Working...", snap.spinner_frame));
        }
    }
    if snap.thinking_level != runie_core::model::ThinkingLevel::Off {
        parts.push(format!("Think: {}", snap.thinking_level.as_str()));
    }
    if !snap.pending_edits.is_empty() {
        parts.push(format!("{} pending", snap.pending_edits.len()));
    }
    if snap.read_only {
        parts.push("🔒 RO".to_string());
    }
    if !snap.auth_providers.is_empty() {
        parts.push(format!("🔑 {}", snap.auth_providers.join(", ")));
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
    let content_width = area.width;
    let lines = build_lines(snap, content_width);
    let offset = snap.scroll_offset(height);
    f.render_widget(
        Paragraph::new(lines).scroll((offset, 0)).wrap(Wrap { trim: false }),
        area,
    );

    if show_bar {
        // Place scrollbar in the right-margin column (past the content area).
        // With a 1-cell margin this sits flush against the terminal edge.
        let full_w = f.area().width;
        let scrollbar_area = Rect {
            x: (area.x + area.width).min(full_w.saturating_sub(1)),
            y: area.y,
            width: 1,
            height: area.height,
        };
        render_scrollbar(f, scrollbar_area, total_lines, offset, height);
    }
}

fn build_lines(snap: &Snapshot, content_width: u16) -> Vec<Line<'_>> {
    let mut lines = Vec::with_capacity(snap.total_lines);
    for elem in snap.elements.iter() {
        lines.extend(to_lines(elem, content_width));
    }
    lines
}

pub fn render_scrollbar(f: &mut Frame, area: Rect, total: usize, offset: u16, height: usize) {
    let scrollbar = Scrollbar::default()
        .orientation(ScrollbarOrientation::VerticalRight)
        .begin_symbol(None)
        .end_symbol(None)
        .track_symbol(Some(SCROLLBAR_TRACK))
        .thumb_symbol(SCROLLBAR_THUMB)
        .style(style_scrollbar());

    // Inverted feed: newest at bottom. offset=0 means top (oldest),
    // offset=max_scroll means bottom (newest). Ratatui's scrollbar
    // thumb reaches the track end only when position == max_position.
    // We achieve this by setting content_length = max_scroll + 1 so
    // max_position = max_scroll, matching our offset range exactly.
    let max_scroll = total.saturating_sub(height);
    let content_length = max_scroll.saturating_add(1);
    let mut state = ScrollbarState::new(content_length)
        .position(offset as usize)
        .viewport_content_length(height);
    f.render_stateful_widget(scrollbar, area, &mut state);
}

fn to_lines<'a>(elem: &'a Element, content_width: u16) -> Vec<Line<'a>> {
    use runie_core::Element::*;
    match elem {
        Spacer { .. } => vec![Line::from("")],
        UserMessage { content, timestamp } => {
            msg::render_user_message(content, *timestamp, content_width)
        }
        AgentMessage { content, timestamp, .. } => {
            msg::render_agent_message(content, *timestamp, content_width)
        }
        Thinking { started, .. } => msg::render_thinking(*started),
        ThoughtSummary { content, duration_secs, .. } => {
            msg::render_thought_summary(content, *duration_secs)
        }
        ThoughtMarker { content, .. } => msg::render_thought_marker(content),
        ToolRunning { name, started, .. } => msg::render_tool_running(name, started.elapsed().as_secs_f64()),
        ToolDone { name, duration_secs, output, .. } => msg::render_tool_done(name, *duration_secs, output),
        ToolSummary { name, duration_secs, .. } => msg::render_tool_summary(name, *duration_secs),
        TurnComplete { duration_secs, .. } => msg::render_turn_complete(*duration_secs),
    }
}

fn count_input_lines(input: &str) -> usize {
    if input.is_empty() {
        return 1;
    }
    let mut lines = input.lines().count().max(1);
    if input.ends_with('\n') {
        lines += 1;
    }
    lines
}

fn input(f: &mut Frame, snap: &Snapshot, area: Rect) {
    let title = format!(" {}/{} ", snap.provider, snap.model);
    let block = block_input(&title, snap.input_flash > 0);
    let token_held = snap.dialog.is_none();
    let lines = build_input_lines(snap, token_held);
    f.render_widget(Paragraph::new(lines).block(block), area);
}

fn build_input_lines(snap: &Snapshot, token_held: bool) -> Vec<Line<'_>> {
    let chevron_style = style_chevron(token_held);
    if snap.input.is_empty() && !snap.placeholder.is_empty() {
        return vec![build_placeholder_line(snap, chevron_style, token_held)];
    }

    let cursor = input_cursor(snap);
    let mut result = build_input_content_lines(snap, cursor, chevron_style, token_held);
    if cursor.line_idx >= snap.input.lines().count() {
        result.push(build_trailing_cursor_line(snap, cursor, chevron_style, token_held));
    }
    result
}

fn build_placeholder_line(snap: &Snapshot, chevron_style: Style, token_held: bool) -> Line<'static> {
    let mut spans = vec![Span::styled(crate::theme::GLYPH_USER, chevron_style)];
    if token_held {
        spans.push(Span::styled(" ".to_string(), style_input_cursor()));
    }
    spans.push(Span::styled(snap.placeholder.clone(), style_placeholder()));
    Line::from(spans)
}

#[derive(Copy, Clone)]
struct InputCursor {
    line_idx: usize,
    col_in_line: usize,
}

fn input_cursor(snap: &Snapshot) -> InputCursor {
    let pos = snap.cursor_pos.min(snap.input.len());
    let line_idx = snap.input[..pos].chars().filter(|&c| c == '\n').count();
    let col_in_line = pos - snap.input.lines().take(line_idx).map(|l| l.len() + 1).sum::<usize>();
    InputCursor { line_idx, col_in_line }
}

fn build_input_content_lines(
    snap: &Snapshot,
    cursor: InputCursor,
    chevron_style: Style,
    token_held: bool,
) -> Vec<Line<'_>> {
    let indent = "  ";
    let mut result = Vec::new();
    for (line_idx, line_content) in snap.input.lines().enumerate() {
        let prefix = if line_idx == 0 { crate::theme::GLYPH_USER } else { indent };
        let mut spans = vec![Span::styled(prefix, chevron_style)];

        if line_idx == cursor.line_idx {
            spans.extend(render_cursor_spans(line_content, cursor.col_in_line, token_held));
        } else {
            spans.push(Span::styled(line_content, crate::theme::style_agent()));
        }

        if line_idx == 0 {
            if let Some(label) = image_attachment_label(snap) {
                spans.push(Span::styled(label, style_hint()));
            }
        }
        result.push(Line::from(spans));
    }
    result
}

fn build_trailing_cursor_line(
    snap: &Snapshot,
    _cursor: InputCursor,
    chevron_style: Style,
    token_held: bool,
) -> Line<'static> {
    let prefix = if snap.input.is_empty() { crate::theme::GLYPH_USER } else { "  " };
    let mut spans = vec![Span::styled(prefix, chevron_style)];
    let cursor_style = if token_held { style_input_cursor() } else { crate::theme::style_agent() };
    spans.push(Span::styled(" ", cursor_style));
    Line::from(spans)
}

fn render_cursor_spans(line_content: &str, cursor_col_in_line: usize, token_held: bool) -> Vec<Span<'_>> {
    let cursor_style = if token_held { style_input_cursor() } else { crate::theme::style_agent() };
    let before = &line_content[..cursor_col_in_line.min(line_content.len())];
    let (at_cursor, after) = if cursor_col_in_line < line_content.len() {
        let c = line_content[cursor_col_in_line..].chars().next().unwrap();
        let char_len = c.len_utf8();
        (c.to_string(), &line_content[cursor_col_in_line + char_len..])
    } else {
        (" ".to_string(), "")
    };
    vec![
        Span::styled(before, crate::theme::style_agent()),
        Span::styled(at_cursor, cursor_style),
        Span::styled(after, crate::theme::style_agent()),
    ]
}

fn image_attachment_label(snap: &Snapshot) -> Option<String> {
    match snap.image_attachments.len() {
        0 => None,
        1 => Some(" 📎 1 image".to_string()),
        n => Some(format!(" 📎 {} images", n)),
    }
}

fn hints(f: &mut Frame, snap: &Snapshot, area: Rect) {
    if let Some(ref msg) = snap.transient_message {
        let (label, bg) = match snap.transient_level {
            Some(runie_core::event::TransientLevel::Success) => ("\\ok\\", crate::theme::color_success()),
            Some(runie_core::event::TransientLevel::Warning) => ("\\warn\\", crate::theme::color_warning()),
            Some(runie_core::event::TransientLevel::Error) => ("\\err\\", crate::theme::color_error()),
            _ => ("", crate::theme::color_bg_panel()),
        };
        let badge_bg = crate::theme::darken(bg, 0.8);
        let margin_bg = crate::theme::darken(bg, 0.85);
        let dark_text = color_bg();
        let margin_style = Style::default().fg(dark_text).bg(margin_bg);
        let msg_style = Style::default().fg(dark_text).bg(bg);
        let badge_style = Style::default().fg(dark_text).bg(badge_bg).bold();
        let content_len = label.len() + 2 + msg.len();
        let fill_len = (area.width as usize).saturating_sub(content_len + 1);
        let fill = " ".repeat(fill_len.max(1));
        let spans = vec![
            Span::styled(" ", margin_style),
            Span::styled(label, badge_style),
            Span::styled(" ", margin_style),
            Span::styled(format!(" {}", msg), msg_style),
            Span::styled(&fill, msg_style),
        ];
        let block = Block::default().borders(Borders::NONE).style(margin_style);
        f.render_widget(Paragraph::new(Line::from(spans)).block(block), area);
    } else {
        let line = Line::from(parse_hint_spans(&snap.hint_text));
        f.render_widget(Paragraph::new(line), area);
    }
}

pub(crate) fn parse_hint_spans(text: &str) -> Vec<Span<'_>> {
    let mut spans = Vec::new();
    let segments: Vec<&str> = text.split(" · ").collect();
    for (i, segment) in segments.iter().enumerate() {
        if let Some(space_idx) = segment.find(' ') {
            let key = &segment[..space_idx];
            let desc = &segment[space_idx..];
            spans.push(Span::styled(key.to_string(), style_hint_key()));
            spans.push(Span::styled(desc.to_string(), style_hint()));
        } else {
            spans.push(Span::styled(segment.to_string(), style_hint()));
        }
        if i + 1 < segments.len() {
            spans.push(Span::styled(" · ".to_string(), style_hint()));
        }
    }
    spans
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
