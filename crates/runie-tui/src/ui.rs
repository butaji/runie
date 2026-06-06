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
    let count = state.count();
    if height == 0 || count == 0 { return; }

    let scroll = count.saturating_sub(height);
    let visible = state.visible(scroll, height);

    let mut lines = Vec::with_capacity(height);
    for elem in visible {
        lines.push(to_line(elem, state));
    }

    f.render_widget(Paragraph::new(lines), inner);
}

fn to_line<'a>(elem: &'a Element, state: &'a AppState) -> Line<'a> {
    use runie_core::Element::*;
    match elem {
        Spacer => Line::from(""),
        UserMessage { content } => Line::from(span(format!("You: {}", content), Color::White)),
        AgentMessage { content } => Line::from(span(format!("Agent: {}", content), Color::White)),
        Thinking { elapsed } => gray(thinking_text(state, *elapsed)),
        ThoughtMarker { content } => gray(Line::from(content.clone())),
        ToolRunning { name, elapsed } => gray(Line::from(format!("{} Running {}... {:.1}s", state.spinner_frame(), name, elapsed))),
        ToolDone { name, duration_secs } => gray(Line::from(format!("◆ Ran {} {:.1}s", name, duration_secs))),
        TurnComplete { duration_secs } => gray(Line::from(format!("Turn completed in {:.1}s", duration_secs))),
        Group { elements, .. } => Line::from(elements.iter().map(|e| to_line(e, state).to_string()).collect::<Vec<_>>().join("\n")),
    }
}

fn span(text: String, color: Color) -> ratatui::text::Span<'static> {
    ratatui::text::Span::styled(text, Style::default().fg(color))
}

fn gray(line: Line<'static>) -> Line<'static> {
    line.style(Style::default().fg(Color::DarkGray))
}

fn thinking_text(state: &AppState, elapsed: f64) -> Line<'static> {
    Line::from(format!("{} Though... {:.1}s", state.spinner_frame(), elapsed))
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
