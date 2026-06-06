//! View — O(visible) rendering, auto-scroll to bottom
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Style},
    text::Line,
    widgets::{Block, Borders, Paragraph},
    Frame,
};

use runie_core::{AppState, PANEL_CHAT, PANEL_INPUT};

pub fn view(f: &mut Frame, state: &AppState) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(3), Constraint::Length(1), Constraint::Length(3)])
        .split(f.area());

    messages_view(f, state, chunks[0]);
    status_view(f, state, chunks[1]);
    input_view(f, state, chunks[2]);
}

fn status_view(f: &mut Frame, state: &AppState, area: Rect) {
    if !state.turn_active { return; }
    let text = format!(
        " {} Working {:.1}s",
        state.spinner_frame(),
        state.turn_elapsed_secs().unwrap_or(0.0)
    );
    f.render_widget(
        Paragraph::new(text).style(Style::default().fg(Color::DarkGray)),
        area,
    );
}

/// O(visible) — only iterate messages that fit on screen, from bottom
fn messages_view(f: &mut Frame, state: &AppState, area: Rect) {
    let block = Block::default()
        .borders(Borders::ALL)
        .title(PANEL_CHAT)
        .border_style(Style::default().fg(Color::DarkGray));
    let inner = block.inner(area);
    f.render_widget(block, area);

    let height = inner.height as usize;
    if height == 0 { return; }

    // Build lines from END, stop when screen is full
    let mut lines: Vec<Line> = Vec::with_capacity(height);

    // Thinking indicator (at bottom during streaming)
    if state.thinking_started_at.is_some() {
        lines.push(Line::from(format!(
            "{} Though... {:.1}s",
            state.spinner_frame(),
            state.thinking_elapsed_secs().unwrap_or(0.0)
        )));
    }

    // Iterate messages from end, stop when we have enough lines
    for msg in state.messages.iter().rev() {
        let line = match msg.role.as_str() {
            "user" => format!("You: {}", msg.content),
            "assistant" => format!("Agent: {}", msg.content),
            "thought" | "tool" | "turn_complete" => msg.content.clone(),
            _ => continue,
        };
        lines.push(Line::from(line));
        lines.push(Line::from("")); // empty line spacer
        if lines.len() >= height { break; }
    }

    lines.reverse(); // bottom-up → top-down

    f.render_widget(
        Paragraph::new(lines).style(Style::default().fg(Color::White)),
        inner,
    );
}

fn input_view(f: &mut Frame, state: &AppState, area: Rect) {
    let block = Block::default()
        .borders(Borders::ALL)
        .title(PANEL_INPUT)
        .border_style(Style::default().fg(Color::DarkGray));
    let inner = block.inner(area);
    f.render_widget(Paragraph::new(state.input.as_str()).block(block), area);
    f.set_cursor_position((inner.x + state.input.len() as u16, inner.y));
}
