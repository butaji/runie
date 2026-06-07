//! View — renders AppState to terminal via ratatui
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Style},
    text::Line,
    widgets::{Block, Borders, Paragraph, Wrap},
    Frame,
};

use runie_core::{AppState, Element, PANEL_CHAT, PANEL_INPUT};

pub fn view(f: &mut Frame, state: &mut AppState) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(3), Constraint::Length(1), Constraint::Length(3)])
        .split(f.area());

    messages(f, state, chunks[0]);
    status(f, state, chunks[1]);
    input(f, state, chunks[2]);
    at_suggestions(f, state);
}

fn status(f: &mut Frame, state: &AppState, area: Rect) {
    let tokens = state.total_tokens();
    let queue_len = state.message_queue.len();
    let mut left_parts = Vec::new();
    if state.turn_active {
        left_parts.push(format!(
            "{} Working {:.1}s",
            state.spinner_frame(),
            state.turn_elapsed_secs().unwrap_or(0.0)
        ));
    }
    if queue_len > 0 {
        left_parts.push(format!("Queue: {}", queue_len));
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

fn messages(f: &mut Frame, state: &mut AppState, area: Rect) {
    let block = Block::default()
        .borders(Borders::ALL)
        .title(PANEL_CHAT)
        .border_style(Style::default().fg(Color::DarkGray));
    let inner = block.inner(area);
    f.render_widget(block, area);


    state.ensure_fresh();

    let height = inner.height as usize;
    let count = state.count();
    if height == 0 || count == 0 { return; }

    let scroll = count.saturating_sub(height);
    let visible = state.visible(scroll, height);

    let mut lines = Vec::with_capacity(height);
    for elem in visible {
        lines.extend(to_lines(elem, state));
    }

    f.render_widget(Paragraph::new(lines).wrap(Wrap { trim: false }), inner);
}

fn to_lines<'a>(elem: &'a Element, state: &'a AppState) -> Vec<Line<'a>> {
    use runie_core::Element::*;
    match elem {
        Spacer => vec![Line::from("")],
        UserMessage { content } => vec![Line::from(span(format!("You: {}", content), Color::White))],
        AgentMessage { content } => vec![Line::from(span(format!("Agent: {}", content), Color::White))],
        Thinking { started } => vec![gray(thinking_text(state, started.elapsed().as_secs_f64()))],
        ThoughtMarker { content } => content.lines().map(|line| gray(Line::from(line.to_string()))).collect(),
        ToolRunning { name, started } => vec![gray(Line::from(format!("{} Running {}... {:.1}s", state.spinner_frame(), name, started.elapsed().as_secs_f64())))],
        ToolDone { name, duration_secs } => vec![gray(Line::from(format!("◆ Ran {} {:.1}s", name, duration_secs)))],
        TurnComplete { duration_secs } => vec![gray(Line::from(format!("Turn completed in {:.1}s", duration_secs)))],
    }

}

fn span(text: String, color: Color) -> ratatui::text::Span<'static> {
    ratatui::text::Span::styled(text, Style::default().fg(color))
}

fn gray(line: Line<'static>) -> Line<'static> {
    line.style(Style::default().fg(Color::DarkGray))
}

fn thinking_text(state: &AppState, elapsed: f64) -> Line<'static> {
    Line::from(format!("{} Thinking... {:.1}s", state.spinner_frame(), elapsed))
}

fn input(f: &mut Frame, state: &AppState, area: Rect) {
    let title = if state.input.contains('@') || state.at_suggestions.is_some() {
        format!("{} @ref (Tab=cycle Enter=insert Esc=close)", PANEL_INPUT)
    } else {
        PANEL_INPUT.to_string()
    };
    let block = Block::default()
        .borders(Borders::ALL)
        .title(title)
        .border_style(Style::default().fg(Color::DarkGray));
    let inner = block.inner(area);
    f.render_widget(Paragraph::new(state.input.as_str()).block(block), area);
    f.set_cursor_position((inner.x + state.input.len() as u16, inner.y));
}

fn at_suggestions(f: &mut Frame, state: &AppState) {
    let suggestions = match &state.at_suggestions {
        Some(s) if !s.is_empty() => s,
        _ => return,
    };
    let selected = state.at_selected.unwrap_or(0).min(suggestions.len().saturating_sub(1));
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
