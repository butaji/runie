//! View — renders Snapshot to terminal via ratatui
//!
//! Architecture: the event loop builds immutable Snapshots;
//! the render actor draws them. No state mutations, no blocking
//! I/O, no caching — pure functions from Snapshot to Frame.

use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Style},
    text::Line,
    widgets::{Block, Borders, Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState, Wrap},
    Frame,
};

use runie_core::{Element, Snapshot, PANEL_CHAT, PANEL_INPUT};

/// Draw a Snapshot to the terminal. Pure function — no mutable state.
pub fn draw_snapshot(f: &mut Frame, snap: &Snapshot) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(3), Constraint::Length(1), Constraint::Length(3), Constraint::Length(1)])
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
    let mut left_parts = Vec::new();
    if snap.turn_active {
        if let Some(elapsed) = snap.turn_elapsed_secs {
            left_parts.push(runie_core::labels::action_text(snap.spinner_frame, "Working", elapsed));
        } else {
            left_parts.push(format!("{} Working...", snap.spinner_frame));
        }
    }
    let left_text = left_parts.join(" | ");
    let right_text = format!("{} tok", tokens);

    let hchunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Min(0), Constraint::Length(right_text.len() as u16)])
        .split(area);

    f.render_widget(
        Paragraph::new(left_text).style(Style::default().fg(Color::DarkGray)),
        hchunks[0],
    );
    f.render_widget(
        Paragraph::new(right_text).style(Style::default().fg(Color::DarkGray)),
        hchunks[1],
    );
}

fn messages(f: &mut Frame, snap: &Snapshot, area: Rect) {
    let block = Block::default()
        .borders(Borders::ALL)
        .title(PANEL_CHAT)
        .border_style(Style::default().fg(Color::DarkGray));
    let inner = block.inner(area);
    f.render_widget(block, area);

    let height = inner.height as usize;
    let total_lines = snap.total_lines;
    if height == 0 || total_lines == 0 { return; }

    let show_bar = total_lines > height;
    let content_width = if show_bar { inner.width.saturating_sub(1) } else { inner.width };

    let hchunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Length(content_width), Constraint::Min(0)])
        .split(inner);

    // Build ALL lines — let ratatui Paragraph::scroll() handle the viewport.
    let mut lines = Vec::with_capacity(total_lines);
    for elem in &snap.elements {
        lines.extend(to_lines(elem, snap.spinner_frame));
    }

    let offset = snap.scroll_offset(height);
    f.render_widget(
        Paragraph::new(lines)
            .scroll((offset, 0))
            .wrap(Wrap { trim: false }),
        hchunks[0],
    );

    if show_bar {
        let scrollbar = Scrollbar::default()
            .orientation(ScrollbarOrientation::VerticalRight)
            .begin_symbol(None)
            .end_symbol(None)
            .track_symbol(Some("│"))
            .thumb_symbol("█");

        let mut scrollbar_state = ScrollbarState::new(total_lines)
            .position(offset as usize)
            .viewport_content_length(height);
        f.render_stateful_widget(scrollbar, inner, &mut scrollbar_state);
    }
}

fn to_lines<'a>(elem: &'a Element, spinner_frame: char) -> Vec<Line<'a>> {
    use runie_core::Element::*;
    match elem {
        Spacer => vec![Line::from("")],
        UserMessage { content } => vec![Line::from(span(format!("You: {}", content), Color::White))],
        AgentMessage { content } => vec![Line::from(span(format!("Agent: {}", content), Color::White))],
        Thinking { started } => vec![gray(Line::from(runie_core::labels::action_text(spinner_frame, "Thinking", started.elapsed().as_secs_f64())))],
        ThoughtSummary { content, .. } => vec![gray(Line::from(
            format!("{} [+]", content.lines().next().unwrap_or(content))
        ))],
        ThoughtMarker { content } => content.lines().map(|line| gray(Line::from(line.to_string()))).collect(),
        ToolRunning { name, started } => vec![gray(Line::from(format!("{} Running {}... {:.1}s", spinner_frame, name, started.elapsed().as_secs_f64())))],
        ToolDone { name, duration_secs, output } => {
            let mut lines = vec![gray(Line::from(format!("◆ Ran {} {:.1}s", name, duration_secs)))];
            if !output.is_empty() {
                for line in output.lines() {
                    lines.push(gray(Line::from(line.to_string())));
                }
            }
            lines
        }
        ToolSummary { name, duration_secs } => vec![gray(Line::from(format!("◆ Ran {} {:.1}s [+]", name, duration_secs)))],
        TurnComplete { duration_secs } => vec![gray(Line::from(format!("Turn completed in {:.1}s", duration_secs)))],
    }
}

fn span(text: String, color: Color) -> ratatui::text::Span<'static> {
    ratatui::text::Span::styled(text, Style::default().fg(color))
}

fn gray(line: Line<'static>) -> Line<'static> {
    line.style(Style::default().fg(Color::DarkGray))
}



fn input(f: &mut Frame, snap: &Snapshot, area: Rect) {
    let block = Block::default()
        .borders(Borders::ALL)
        .title(PANEL_INPUT)
        .border_style(Style::default().fg(Color::DarkGray));
    let inner = block.inner(area);
    f.render_widget(Paragraph::new(snap.input.as_str()).block(block), area);
    f.set_cursor_position((inner.x + snap.input.len() as u16, inner.y));
}

fn hints(f: &mut Frame, snap: &Snapshot, area: Rect) {
    f.render_widget(
        Paragraph::new(snap.hint_text.as_str()).style(Style::default().fg(Color::DarkGray)),
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
    let mut lines: Vec<Line> = suggestions.iter().take(8).enumerate().map(|(i, s)| {
        let prefix = if i == selected { "▸ " } else { "  " };
        let style = if i == selected {
            Style::default().fg(Color::Black).bg(Color::Yellow)
        } else {
            Style::default().fg(Color::White)
        };
        Line::from(format!("{}{}", prefix, s)).style(style)
    }).collect();
    lines.push(Line::from(""));
    lines.push(Line::from("Tab=cycle Enter=insert Esc=close").style(Style::default().fg(Color::DarkGray)));
    let block = Block::default()
        .borders(Borders::ALL)
        .title(format!(" @ files ({}) ", suggestions.len()))
        .border_style(Style::default().fg(Color::Magenta));
    f.render_widget(Paragraph::new(lines).block(block), popup_area);
}

fn estimate_element_tokens(elem: &Element) -> usize {
    use runie_core::Element::*;
    match elem {
        UserMessage { content } | AgentMessage { content } | ThoughtMarker { content } => content.len() / 4,
        Thinking { .. } | ThoughtSummary { .. } | ToolSummary { .. } | TurnComplete { .. } => 10,
        ToolRunning { .. } => 10,
        ToolDone { output, .. } => output.len() / 4 + 10,
        Spacer => 0,
    }
}
