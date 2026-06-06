//! View - Terminal Rendering
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::Style,
    text::Line,
    widgets::Paragraph,
    Frame,
};

use runie_core::{AppState, Dsl, PANEL_CHAT, PANEL_INPUT, Element};

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
    let block = ratatui::widgets::Block::default()
        .borders(ratatui::widgets::Borders::ALL)
        .title(PANEL_CHAT)
        .border_style(Style::default().fg(ratatui::style::Color::DarkGray))
        .title_style(Style::default().fg(ratatui::style::Color::DarkGray));
    let inner = block.inner(area);
    f.render_widget(block, area);

    let total = Dsl::count(state);
    let visible_height = inner.height as usize;
    
    if total == 0 || visible_height == 0 {
        return;
    }

    // Auto-scroll to bottom - only show last visible_height elements
    let scroll = if total > visible_height {
        total - visible_height
    } else {
        0
    };
    
    // Get only visible elements - O(visible) not O(total)
    let elements = Dsl::visible(state, scroll, visible_height);
    
    // Build lines for visible only
    let width = inner.width as usize;
    let mut lines: Vec<Line> = Vec::with_capacity(visible_height);
    
    for elem in &elements {
        let text = element_to_text(elem, state);
        if text.is_empty() {
            lines.push(Line::from(""));
        } else {
            for line in wrap_text(&text, width) {
                lines.push(Line::from(line));
                if lines.len() >= visible_height {
                    break;
                }
            }
        }
        if lines.len() >= visible_height {
            break;
        }
    }
    
    // Render with Paragraph
    let paragraph = Paragraph::new(lines)
        .style(Style::default().fg(ratatui::style::Color::White));
    f.render_widget(paragraph, inner);
}

fn wrap_text(text: &str, width: usize) -> Vec<String> {
    if width == 0 {
        return vec![text.to_string()];
    }
    
    let mut result = Vec::new();
    for paragraph in text.lines() {
        let mut remaining = paragraph;
        while !remaining.is_empty() {
            if remaining.len() <= width {
                result.push(remaining.to_string());
                break;
            }
            let cut = remaining[..width.min(remaining.len())]
                .rfind(' ')
                .map(|i| i)
                .unwrap_or(width.min(remaining.len()));
            result.push(remaining[..cut].to_string());
            remaining = remaining[cut..].trim_start();
        }
    }
    result
}

fn element_to_text(element: &Element, state: &AppState) -> String {
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
            elements.iter()
                .map(|e| element_to_text(e, state))
                .collect::<Vec<_>>()
                .join("\n")
        }
    }
}

fn input_view(f: &mut Frame, state: &AppState, area: Rect) {
    let block = ratatui::widgets::Block::default()
        .borders(ratatui::widgets::Borders::ALL)
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
