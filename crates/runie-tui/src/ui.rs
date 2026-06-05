//! View - Terminal Rendering
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Style},
    text::{Line, Span, Text},
    widgets::{Block, Borders, Paragraph, Wrap},
    Frame,
};

use runie_core::AppState;

/// View function - renders state to terminal
/// Takes immutable state, returns rendered UI
pub fn view(f: &mut Frame, state: &AppState) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(3), Constraint::Length(3)])
        .split(f.area());

    messages_view(f, state, chunks[0]);
    input_view(f, state, chunks[1]);
}

fn messages_view(f: &mut Frame, state: &AppState, area: Rect) {
    let block = Block::default()
        .borders(Borders::ALL)
        .title(" Chat ")
        .border_style(Style::default().fg(Color::DarkGray))
        .title_style(Style::default().fg(Color::DarkGray));
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

fn message_to_lines(msg: &runie_core::ChatMessage) -> Vec<Line<'_>> {
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

fn input_view(f: &mut Frame, state: &AppState, area: Rect) {
    // Determine title and content based on state
    let (title, content, border_style) = if state.streaming {
        let elapsed = state.thinking_elapsed_secs()
            .map(|s| format!(" Thinking... {:.1}s ", s))
            .unwrap_or_else(|| " Thinking... ".to_string());
        (elapsed, String::new(), Style::default().fg(Color::Yellow))
    } else {
        (" Input ".to_string(), state.input.clone(), Style::default().fg(Color::DarkGray))
    };
    
    let block = Block::default()
        .borders(Borders::ALL)
        .title(title)
        .border_style(border_style)
        .title_style(Style::default().fg(Color::DarkGray));
    let inner = block.inner(area);
    let paragraph = Paragraph::new(content).block(block);
    f.render_widget(paragraph, area);
    
    // Position cursor at end of input (only when not streaming)
    if !state.streaming {
        let cursor_x = (inner.x + state.input.len() as u16).min(inner.right() - 1);
        let cursor_y = inner.y;
        f.set_cursor_position((cursor_x, cursor_y));
    }
}
