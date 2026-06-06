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
    f.render_widget(&block, area);

    // Skip rendering if nothing to show
    let total = Dsl::count(state);
    if total == 0 || inner.height == 0 {
        return;
    }

    // Get visible elements - O(take)
    let height = inner.height as usize;
    let scroll = total.saturating_sub(height);
    let visible = Dsl::visible(state, scroll, height);
    
    // Render directly to buffer
    let buf = f.buffer_mut();
    for (i, elem) in visible.iter().enumerate() {
        if i >= height {
            break;
        }
        let text = element_to_line(elem, state);
        buf.set_line(inner.x, inner.y + i as u16, &text, inner.width);
    }
}

fn element_to_line<'a>(element: &'a Element, state: &'a AppState) -> Line<'a> {
    use runie_core::Element;
    
    match element {
        Element::Spacer => Line::from(""),
        Element::UserMessage { content } => Line::from(format!("You: {}", content)),
        Element::AgentMessage { content } => Line::from(format!("Agent: {}", content)),
        Element::Thinking { elapsed } => {
            Line::from(format!("{} Though... {:.1}s", state.spinner_frame(), elapsed))
        }
        Element::ThoughtMarker { content } => Line::from(content.as_str()),
        Element::ToolRunning { name, elapsed } => {
            Line::from(format!("{} Running {}... {:.1}s", state.spinner_frame(), name, elapsed))
        }
        Element::ToolDone { name, duration_secs } => {
            Line::from(format!("◆ Ran {} {:.1}s", name, duration_secs))
        }
        Element::TurnComplete { duration_secs } => {
            Line::from(format!("Turn completed in {:.1}s", duration_secs))
        }
        Element::Group { elements, .. } => {
            let text = elements.iter()
                .map(|e| element_to_text(e, state))
                .collect::<Vec<_>>()
                .join("\n");
            Line::from(text)
        }
    }
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
