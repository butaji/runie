//! View - Terminal Rendering
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::Style,
    text::{Line, Span, Text},
    widgets::{Block, Borders, Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState},
    Frame,
};

use runie_core::{AppState, format_messages, thinking, PANEL_CHAT, PANEL_INPUT, Color as CoreColor};

/// View function - renders state to terminal
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
        .title(PANEL_CHAT)
        .border_style(Style::default().fg(ratatui::style::Color::DarkGray))
        .title_style(Style::default().fg(ratatui::style::Color::DarkGray));
    let inner = block.inner(area);
    f.render_widget(block, area);

    // Get formatted messages (without thinking indicator)
    let lines = format_messages(state);
    
    // Convert display lines to ratatui lines
    let ratatui_lines: Vec<Line> = lines
        .into_iter()
        .map(|dl| {
            if dl.spans.is_empty() {
                Line::raw("")
            } else {
                let spans: Vec<Span> = dl.spans
                    .into_iter()
                    .map(|s| {
                        let color = s.color.map(cratatui_color);
                        match color {
                            Some(c) => Span::styled(s.text, Style::default().fg(c)),
                            None => Span::raw(s.text),
                        }
                    })
                    .collect();
                Line::from(spans)
            }
        })
        .collect();

    let content_height = ratatui_lines.len() as u16;
    let visible_height = inner.height;
    let has_thinking = state.streaming || !state.request_queue.is_empty();
    
    // Calculate scroll - show bottom if content overflows
    let scroll_offset = if content_height > visible_height {
        content_height.saturating_sub(visible_height) as usize
    } else {
        0
    };

    // Get visible lines
    let visible_lines: Vec<Line> = if scroll_offset > 0 {
        ratatui_lines[scroll_offset..].to_vec()
    } else {
        ratatui_lines.clone()
    };

    let paragraph = Paragraph::new(Text::from(visible_lines));
    f.render_widget(paragraph, inner);

    // Add scrollbar if content is scrollable
    if content_height > visible_height {
        let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight)
            .thumb_style(Style::default().fg(ratatui::style::Color::DarkGray));
        let mut scrollbar_state = ScrollbarState::new(content_height as usize)
            .position(scroll_offset);
        f.render_stateful_widget(scrollbar, inner, &mut scrollbar_state);
    }

    // Render thinking indicator (overwrite last line of messages if present)
    if has_thinking {
        let thinking_lines = thinking(state);
        let thinking_text: Vec<Line> = thinking_lines
            .into_iter()
            .map(|dl| {
                if dl.spans.is_empty() {
                    Line::raw("")
                } else {
                    let spans: Vec<Span> = dl.spans
                        .into_iter()
                        .map(|s| {
                            let color = s.color.map(cratatui_color);
                            match color {
                                Some(c) => Span::styled(s.text, Style::default().fg(c)),
                                None => Span::raw(s.text),
                            }
                        })
                        .collect();
                    Line::from(spans)
                }
            })
            .collect();
        
        // Position at bottom of inner area
        let y = inner.y + inner.height.saturating_sub(1);
        let thinking_area = Rect::new(inner.x, y, inner.width, 1);
        let paragraph = Paragraph::new(Text::from(thinking_text));
        f.render_widget(paragraph, thinking_area);
    }
}

fn input_view(f: &mut Frame, state: &AppState, area: Rect) {
    let block = Block::default()
        .borders(Borders::ALL)
        .title(PANEL_INPUT)
        .border_style(Style::default().fg(ratatui::style::Color::DarkGray))
        .title_style(Style::default().fg(ratatui::style::Color::DarkGray));
    let inner = block.inner(area);
    let paragraph = Paragraph::new(state.input.as_str()).block(block);
    f.render_widget(paragraph, area);
    
    // Position cursor at end of input
    let cursor_x = (inner.x + state.input.len() as u16).min(inner.right() - 1);
    let cursor_y = inner.y;
    f.set_cursor_position((cursor_x, cursor_y));
}

fn cratatui_color(c: CoreColor) -> ratatui::style::Color {
    match c {
        CoreColor::Cyan => ratatui::style::Color::Cyan,
        CoreColor::Green => ratatui::style::Color::Green,
        CoreColor::Yellow => ratatui::style::Color::Yellow,
        CoreColor::DarkGray => ratatui::style::Color::DarkGray,
        CoreColor::White => ratatui::style::Color::White,
    }
}
