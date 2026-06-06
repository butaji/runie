//! View - Terminal Rendering
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::Style,
    text::{Line, Span, Text},
    widgets::{Block, Borders, Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState, Wrap},
    Frame,
};

use runie_core::{AppState, format_messages, PANEL_CHAT, PANEL_INPUT, Color as CoreColor};

/// View function - renders state to terminal
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
    use ratatui::style::Style;
    
    if state.turn_active {
        let spinner = state.spinner_frame();
        let elapsed = state.turn_elapsed_secs().unwrap_or(0.0);
        let text = format!(" {} Working {:.1}s", spinner, elapsed);
        
        let paragraph = Paragraph::new(text)
            .style(Style::default().fg(ratatui::style::Color::DarkGray));
        f.render_widget(paragraph, area);
    }
}

fn messages_view(f: &mut Frame, state: &AppState, area: Rect) {
    // Split off 1 column for scrollbar track on the right
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Fill(1), Constraint::Length(1)])
        .split(area);
    
    let content_area = chunks[0];
    let scroll_area = chunks[1];
    
    let block = Block::default()
        .borders(Borders::ALL)
        .title(PANEL_CHAT)
        .border_style(Style::default().fg(ratatui::style::Color::DarkGray))
        .title_style(Style::default().fg(ratatui::style::Color::DarkGray));
    let inner = block.inner(content_area);
    f.render_widget(block, content_area);

    let lines = format_messages(state);
    
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
    
    // Calculate scroll position (auto-scroll to bottom)
    let max_scroll = content_height.saturating_sub(visible_height) as usize;
    let scroll_offset = if content_height > visible_height {
        max_scroll
    } else {
        0
    };

    // Render content with scroll offset applied
    let visible_lines: Vec<Line> = if scroll_offset > 0 {
        ratatui_lines[scroll_offset..].to_vec()
    } else {
        ratatui_lines
    };

    let paragraph = Paragraph::new(Text::from(visible_lines))
        .wrap(Wrap { trim: true });
    f.render_widget(paragraph, inner);

    // Render scrollbar in the track area
    if content_height > visible_height {
        let mut scrollbar_state = ScrollbarState::new(content_height as usize)
            .position(scroll_offset);
        let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight)
            .thumb_style(Style::default().fg(ratatui::style::Color::DarkGray));
        f.render_stateful_widget(scrollbar, scroll_area, &mut scrollbar_state);
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
        CoreColor::Magenta => ratatui::style::Color::Magenta,
    }
}
