//! View - Terminal Rendering
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::Style,
    text::Text,
    widgets::{Block, Borders, List, ListItem, Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState},
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
    // Split content / scrollbar track
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

    // Get formatted lines from cache
    let lines = format_messages(state);
    let total_lines = lines.len();
    let visible_height = inner.height as usize;
    
    // Calculate visible window - only render what's on screen
    let max_scroll = total_lines.saturating_sub(visible_height);
    let scroll_offset = if total_lines > visible_height {
        max_scroll  // Auto-scroll to bottom
    } else {
        0
    };

    // Create visible items using skip().take() - O(1) per item
    let visible_lines: Vec<ListItem> = lines
        .iter()
        .skip(scroll_offset)
        .take(visible_height)
        .map(|dl| {
            // Use raw text without complex styling for efficiency
            let text = dl.spans.iter().map(|s| s.text.clone()).collect::<String>();
            ListItem::new(text)
        })
        .collect();

    // Render visible window only
    let list = List::new(visible_lines);
    f.render_widget(list, inner);

    // Render scrollbar - O(1) state
    if total_lines > visible_height {
        let mut scrollbar_state = ScrollbarState::new(total_lines)
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
