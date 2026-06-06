//! View - Terminal Rendering (Minimal & Fast)
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Style, Color},
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
    let text = format!(" {} Working {:.1}s", state.spinner_frame(), state.turn_elapsed_secs().unwrap_or(0.0));
    f.render_widget(Paragraph::new(text).style(Style::default().fg(Color::DarkGray)), area);
}

fn messages_view(f: &mut Frame, state: &AppState, area: Rect) {
    let block = Block::default().borders(Borders::ALL).title(PANEL_CHAT)
        .border_style(Style::default().fg(Color::DarkGray));
    let inner = block.inner(area);
    f.render_widget(block, area);

    if inner.height == 0 { return; }

    // Build all lines
    let mut lines = Vec::new();

    // Thinking
    if state.thinking_started_at.is_some() {
        lines.push(Line::from(format!("{} Though... {:.1}s", state.spinner_frame(), state.thinking_elapsed_secs().unwrap_or(0.0))));
    }

    // Messages
    for msg in &state.messages {
        let line = match msg.role.as_str() {
            "user" => format!("You: {}", msg.content),
            "assistant" => format!("Agent: {}", msg.content),
            "thought" => msg.content.clone(),
            "tool" => msg.content.clone(),
            "turn_complete" => msg.content.clone(),
            _ => continue,
        };
        lines.push(Line::from(line));
    }

    // Auto-scroll: last N lines
    let start = lines.len().saturating_sub(inner.height as usize);
    let visible = &lines[start..];

    f.render_widget(
        Paragraph::new(visible.to_vec()).style(Style::default().fg(Color::White)),
        inner,
    );
}

fn input_view(f: &mut Frame, state: &AppState, area: Rect) {
    let block = Block::default().borders(Borders::ALL).title(PANEL_INPUT)
        .border_style(Style::default().fg(Color::DarkGray));
    let inner = block.inner(area);
    f.render_widget(Paragraph::new(state.input.as_str()).block(block), area);
    f.set_cursor_position((inner.x + state.input.len() as u16, inner.y));
}
