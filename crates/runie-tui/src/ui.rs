//! View - Terminal Rendering
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::Style,
    widgets::{Block, Borders, List, ListItem, Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState},
    Frame,
};

use runie_core::{AppState, Dsl, PANEL_CHAT, PANEL_INPUT, Color as CoreColor};

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
    let [content_area, scroll_area] = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Fill(1), Constraint::Length(1)])
        .areas(area);
    
    let block = Block::default()
        .borders(Borders::ALL)
        .title(PANEL_CHAT)
        .border_style(Style::default().fg(ratatui::style::Color::DarkGray))
        .title_style(Style::default().fg(ratatui::style::Color::DarkGray));
    let inner = block.inner(content_area);
    f.render_widget(block, content_area);

    let visible_height = inner.height as usize;
    let total_elements = Dsl::count(state);
    
    // Auto-scroll to bottom
    let max_scroll = total_elements.saturating_sub(visible_height);
    let scroll_offset = if total_elements > visible_height {
        max_scroll
    } else {
        0
    };

    // Get ONLY visible elements - O(visible) not O(n)
    let visible_elements = Dsl::visible(state, scroll_offset, visible_height);
    
    // Convert elements to ListItems directly
    let items: Vec<ListItem> = visible_elements.iter().map(|elem| {
        let text = element_to_text(elem, state);
        ListItem::new(text)
    }).collect();

    // Render visible window only
    let list = List::new(items);
    f.render_widget(list, inner);

    // Render scrollbar
    if total_elements > visible_height {
        let mut scrollbar_state = ScrollbarState::new(total_elements)
            .position(scroll_offset);
        let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight)
            .thumb_style(Style::default().fg(ratatui::style::Color::DarkGray));
        f.render_stateful_widget(scrollbar, scroll_area, &mut scrollbar_state);
    }
}

fn element_to_text(element: &runie_core::Element, state: &AppState) -> String {
    use runie_core::Element;
    
    match element {
        Element::Spacer => String::new(),
        Element::UserMessage { content } => format!("You: {}", content),
        Element::AgentMessage { content } => format!("Agent: {}", content),
        Element::Thinking { elapsed } => {
            format!("{} Though... {:.1}s", state.spinner_frame(), elapsed)
        }
        Element::ThoughtMarker { content } => content.clone(),
        Element::ToolRunning { name, elapsed } => {
            format!("{} Running {}... {:.1}s", state.spinner_frame(), name, elapsed)
        }
        Element::ToolDone { name, duration_secs } => {
            format!("◆ Ran {} {:.1}s", name, duration_secs)
        }
        Element::TurnComplete { duration_secs } => {
            format!("Turn completed in {:.1}s", duration_secs)
        }
        Element::Group { elements, .. } => {
            elements.iter().map(|e| element_to_text(e, state)).collect::<Vec<_>>().join("\n")
        }
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
