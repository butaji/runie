//! View — renders AppState to terminal via ratatui
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Style},
    text::Line,
    widgets::{Block, Borders, Paragraph},
    Frame,
};

use runie_core::{AppState, Element, PANEL_CHAT, PANEL_INPUT};

pub fn view(f: &mut Frame, state: &AppState) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(3), Constraint::Length(1), Constraint::Length(3)])
        .split(f.area());

    messages(f, state, chunks[0]);
    status(f, state, chunks[1]);
    input(f, state, chunks[2]);
}

fn status(f: &mut Frame, state: &AppState, area: Rect) {
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

fn messages(f: &mut Frame, state: &AppState, area: Rect) {
    let block = Block::default()
        .borders(Borders::ALL)
        .title(PANEL_CHAT)
        .border_style(Style::default().fg(Color::DarkGray));
    let inner = block.inner(area);
    f.render_widget(block, area);

    let height = inner.height as usize;
    if height == 0 || state.element_count == 0 { return; }

    let scroll = state.element_count.saturating_sub(height);
    let visible = state.visible(scroll, height);

    let mut lines = Vec::with_capacity(height);
    for elem in visible {
        lines.push(to_line(elem, state));
    }

    f.render_widget(Paragraph::new(lines), inner);
}

fn to_line<'a>(elem: &'a Element, state: &'a AppState) -> Line<'a> {
    use runie_core::Element::*;

    let gray = Style::default().fg(Color::DarkGray);
    let white = Style::default().fg(Color::White);

    match elem {
        Spacer => Line::from(""),
        UserMessage { content } => Line::from(
            ratatui::text::Span::styled(format!("You: {}", content), white)
        ),
        AgentMessage { content } => Line::from(
            ratatui::text::Span::styled(format!("Agent: {}", content), white)
        ),
        Thinking { elapsed } => Line::from(
            ratatui::text::Span::styled(
                format!("{} Though... {:.1}s", state.spinner_frame(), elapsed),
                gray,
            )
        ),
        ThoughtMarker { content } => Line::from(
            ratatui::text::Span::styled(content.clone(), gray)
        ),
        ToolRunning { name, elapsed } => Line::from(
            ratatui::text::Span::styled(
                format!("{} Running {}... {:.1}s", state.spinner_frame(), name, elapsed),
                gray,
            )
        ),
        ToolDone { name, duration_secs } => Line::from(
            ratatui::text::Span::styled(
                format!("◆ Ran {} {:.1}s", name, duration_secs),
                gray,
            )
        ),
        TurnComplete { duration_secs } => Line::from(
            ratatui::text::Span::styled(
                format!("Turn completed in {:.1}s", duration_secs),
                gray,
            )
        ),
        Group { elements, .. } => {
            let text: String = elements.iter()
                .map(|e| to_line(e, state).to_string())
                .collect::<Vec<_>>()
                .join("\n");
            Line::from(text)
        }
    }
}

fn input(f: &mut Frame, state: &AppState, area: Rect) {
    let block = Block::default()
        .borders(Borders::ALL)
        .title(PANEL_INPUT)
        .border_style(Style::default().fg(Color::DarkGray));
    let inner = block.inner(area);
    f.render_widget(Paragraph::new(state.input.as_str()).block(block), area);
    f.set_cursor_position((inner.x + state.input.len() as u16, inner.y));
}
