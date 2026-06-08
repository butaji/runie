//! View — renders Snapshot to terminal via ratatui
//!
//! Architecture: the event loop builds immutable Snapshots;
//! the render actor draws them. No state mutations, no blocking
//! I/O, no caching — pure functions from Snapshot to Frame.

use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::Style,
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState, Wrap},
    Frame,
};

use runie_core::{Element, Snapshot, PANEL_CHAT, PANEL_INPUT};

use crate::theme::C;

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
        Style::default().fg(C.success)
    } else {
        Style::default().fg(C.dim)
    };

    f.render_widget(Paragraph::new(left_text).style(status_style), hchunks[0]);
    f.render_widget(
        Paragraph::new(right_text).style(Style::default().fg(C.dim)),
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
    let block = Block::default()
        .borders(Borders::ALL)
        .title(PANEL_CHAT)
        .border_style(Style::default().fg(C.dim));
    let inner = block.inner(area);
    f.render_widget(block, area);

    if snap.elements.is_empty() {
        render_empty_state(f, inner);
        return;
    }
    render_message_content(f, snap, inner);
}

fn render_empty_state(f: &mut Frame, area: Rect) {
    let hint = Line::from("Type a message to start...")
        .style(Style::default().fg(C.dim));
    f.render_widget(
        Paragraph::new(hint).alignment(ratatui::layout::Alignment::Center),
        area,
    );
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
        .track_symbol(Some("│"))
        .thumb_symbol("█");

    let mut state = ScrollbarState::new(total)
        .position(offset as usize)
        .viewport_content_length(height);
    f.render_stateful_widget(scrollbar, area, &mut state);
}

fn to_lines<'a>(elem: &'a Element, _spinner_frame: char) -> Vec<Line<'a>> {
    use runie_core::Element::*;
    match elem {
        Spacer => vec![],
        // Critical #2: Wrapped line indentation with "$ " prefix
        UserMessage { content, timestamp } => render_user_message(content, timestamp),
        AgentMessage { content, timestamp } => render_agent_message(content, timestamp),
        // DS-04: tui1-style thinking indicator
        Thinking { started } => vec![Line::from(format!(
            "{} ◐ {:.1}s", "→", started.elapsed().as_secs_f64()
        )).style(Style::default().fg(C.accent))],
        ThoughtSummary { content, .. } => vec![Line::from(format!(
            "{} [+]", content.lines().next().unwrap_or(content)
        )).style(Style::default().fg(C.dim))],
        ThoughtMarker { content } => render_thought_marker(content),
        // DS-06: Tool calls inline as feed items
        ToolRunning { name, started } => vec![Line::from(format!(
            "✓ Running {}... {:.1}s", name, started.elapsed().as_secs_f64()
        )).style(Style::default().fg(C.fg_mid))],
        ToolDone { name, duration_secs, output } => render_tool_done(name, *duration_secs, output),
        ToolSummary { name, duration_secs } => vec![Line::from(format!(
            "✓ {} {:.1}s [+]", name, duration_secs
        )).style(Style::default().fg(C.dim))],
        TurnComplete { duration_secs } => vec![Line::from(format!(
            "Turn completed in {:.1}s", duration_secs
        )).style(Style::default().fg(C.dim))],
    }
}

fn render_user_message(content: &str, timestamp: &str) -> Vec<Line<'static>> {
    let mut lines = Vec::new();
    let prefix = "$ ";
    for (i, line) in content.lines().enumerate() {
        let text = if i == 0 {
            format!("{}{} {}", prefix, line, timestamp)
        } else {
            format!("  {} {}", line, timestamp)
        };
        lines.push(Line::from(text).style(Style::default().fg(C.fg_bright)));
    }
    if lines.is_empty() {
        lines.push(Line::from(format!("{} {}", prefix, timestamp))
            .style(Style::default().fg(C.fg_bright)));
    }
    lines
}

fn render_agent_message(content: &str, timestamp: &str) -> Vec<Line<'static>> {
    let mut lines = Vec::new();
    let prefix = "→ ";
    for (i, line) in content.lines().enumerate() {
        let text = if i == 0 {
            format!("{}{} {}", prefix, line, timestamp)
        } else {
            format!("  {} {}", line, timestamp)
        };
        lines.push(Line::from(text).style(Style::default().fg(C.fg)));
    }
    if lines.is_empty() {
        lines.push(Line::from(format!("{} {}", prefix, timestamp))
            .style(Style::default().fg(C.fg)));
    }
    lines
}

fn render_thought_marker(content: &str) -> Vec<Line<'static>> {
    content.lines()
        .map(|line| Line::from(line.to_string()).style(Style::default().fg(C.fg_mid)))
        .collect()
}

fn render_tool_done(name: &str, duration_secs: f64, output: &str) -> Vec<Line<'static>> {
    let mut lines = vec![Line::from(format!("✓ {} {:.1}s", name, duration_secs))
        .style(Style::default().fg(C.success))];
    if !output.is_empty() {
        for line in output.lines() {
            lines.push(Line::from(line.to_string()).style(Style::default().fg(C.fg_mid)));
        }
    }
    lines
}

fn input(f: &mut Frame, snap: &Snapshot, area: Rect) {
    let block = Block::default()
        .borders(Borders::ALL)
        .title(PANEL_INPUT)
        .border_style(Style::default().fg(C.dim));
    let inner = block.inner(area);
    
    let spans = build_input_spans(snap);
    f.render_widget(Paragraph::new(Line::from(spans)).block(block), area);
    
    // Position cursor (terminal cursor, for TTY visibility)
    let cursor_x = inner.x + ("$ ".len() + snap.cursor_pos.min(snap.input.len())) as u16;
    f.set_cursor_position((cursor_x, inner.y));
}

fn build_input_spans(snap: &Snapshot) -> Vec<Span> {
    let prefix = "$ ";
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
        Span::styled(prefix, Style::default().fg(C.fg_bright)),
        Span::styled(before, Style::default().fg(C.fg_bright)),
        Span::styled(at_cursor.to_string(), Style::default().bg(C.fg_bright).fg(C.bg)),
        Span::styled(after, Style::default().fg(C.fg_bright)),
    ]
}

fn hints(f: &mut Frame, snap: &Snapshot, area: Rect) {
    let hints_text = if snap.turn_active {
        "Ctrl+Shift+E=expand/collapse | Enter=steer | Alt+Enter=follow-up | Esc=abort | Ctrl+C=quit"
    } else {
        "Ctrl+Shift+E=expand/collapse | Alt+Enter=follow-up | Esc=clear | Ctrl+C=quit"
    };
    f.render_widget(
        Paragraph::new(hints_text).style(Style::default().fg(C.fg)),
        area,
    );
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
            let prefix = if i == selected { "▸ " } else { "  " };
            let style = if i == selected {
                Style::default().fg(C.dim).bg(C.fg_mid)
            } else {
                Style::default().fg(C.fg_mid)
            };
            Line::from(format!("{}{}", prefix, s)).style(style)
        })
        .collect();
    lines.push(Line::from(""));
    lines.push(
        Line::from("Tab=cycle Enter=insert Esc=close")
            .style(Style::default().fg(C.dim)),
    );
    let block = Block::default()
        .borders(Borders::ALL)
        .title(format!(" @ files ({}) ", suggestions.len()))
        .border_style(Style::default().fg(C.accent));
    f.render_widget(Paragraph::new(lines).block(block), popup_area);
}

fn estimate_element_tokens(elem: &Element) -> usize {
    use runie_core::Element::*;
    match elem {
        UserMessage { content, .. }
        | AgentMessage { content, .. }
        | ThoughtMarker { content } => content.len() / 4,
        Thinking { .. }
        | ThoughtSummary { .. }
        | ToolSummary { .. }
        | TurnComplete { .. } => 10,
        ToolRunning { .. } => 10,
        ToolDone { output, .. } => output.len() / 4 + 10,
        Spacer => 0,
    }
}
