//! UI rendering.
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Style},
    text::{Line, Span, Text},
    widgets::{Block, Borders, Paragraph, Wrap},
    Frame,
};

use crate::AppState;

pub fn draw(f: &mut Frame, state: &AppState) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(3), Constraint::Length(3)])
        .split(f.area());

    draw_messages(f, state, chunks[0]);
    draw_input(f, state, chunks[1]);
}

fn draw_messages(f: &mut Frame, state: &AppState, area: Rect) {
    let block = Block::default().borders(Borders::ALL).title(" Chat ");
    let inner = block.inner(area);
    f.render_widget(block, area);

    let visible_height = inner.height as usize;
    let total = state.messages.len();
    if total == 0 {
        return;
    }
    
    let start = state.scroll.saturating_sub(visible_height.saturating_sub(1));
    let end = (start + visible_height).min(total);
    let visible = &state.messages[start..end];

    let lines: Vec<Line> = visible
        .iter()
        .flat_map(message_to_lines)
        .collect();

    let paragraph = Paragraph::new(Text::from(lines))
        .wrap(Wrap { trim: false })
        .scroll((0, 0));
    f.render_widget(paragraph, inner);
}

fn message_to_lines(msg: &crate::ChatMessage) -> Vec<Line<'_>> {
    let (prefix, color) = match msg.role.as_str() {
        "user" => ("You: ", Color::Cyan),
        "assistant" => ("Agent: ", Color::Green),
        _ => ("", Color::White),
    };

    let mut lines = vec![];
    for (i, line_text) in msg.content.lines().enumerate() {
        if i == 0 && !prefix.is_empty() {
            lines.push(Line::from(vec![
                Span::styled(prefix, Style::default().fg(color)),
                Span::raw(line_text),
            ]));
        } else {
            let indent = " ".repeat(prefix.len());
            lines.push(Line::raw(format!("{}{}", indent, line_text)));
        }
    }
    if lines.is_empty() {
        lines.push(Line::raw(""));
    }
    lines.push(Line::raw(""));
    lines
}

fn draw_input(f: &mut Frame, state: &AppState, area: Rect) {
    let block = Block::default()
        .borders(Borders::ALL)
        .title(" Input ")
        .border_style(if state.streaming {
            Style::default().fg(Color::DarkGray)
        } else {
            Style::default().fg(Color::White)
        });
    let paragraph = Paragraph::new(state.input.as_str()).block(block);
    f.render_widget(paragraph, area);
}
