use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Style},
    text::{Line, Span, Text},
    widgets::{Block, Borders, Clear, Paragraph, Wrap},
    Frame,
};
use runie_app::AppState;

pub fn draw(f: &mut Frame, state: &AppState) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(3), Constraint::Length(3)])
        .split(f.area());

    draw_messages(f, state, chunks[0]);
    draw_input(f, state, chunks[1]);

    if state.streaming {
        let area = centered_rect(20, 3, f.area());
        f.render_widget(Clear, area);
        let block = Block::default()
            .borders(Borders::ALL)
            .style(Style::default().fg(Color::Yellow));
        let para = Paragraph::new("Thinking...").block(block);
        f.render_widget(para, area);
    }
}

fn draw_messages(f: &mut Frame, state: &AppState, area: Rect) {
    let block = Block::default().borders(Borders::ALL).title(" Chat ");
    let inner = block.inner(area);
    f.render_widget(block, area);

    let visible_height = inner.height as usize;
    let total = state.messages.len();
    let start = state.scroll.saturating_sub(visible_height.saturating_sub(1));
    let end = (start + visible_height).min(total);
    let visible = &state.messages[start..end];

    let lines: Vec<Line> = visible
        .iter()
        .flat_map(|msg| message_to_lines(msg))
        .collect();

    let paragraph = Paragraph::new(Text::from(lines))
        .wrap(Wrap { trim: false })
        .scroll((0, 0));
    f.render_widget(paragraph, inner);
}

fn message_to_lines(msg: &runie_app::ChatMessage) -> Vec<Line<'_>> {
    let (prefix, color) = match msg.role.as_str() {
        "user" => ("You: ", Color::Cyan),
        "assistant" => ("Agent: ", Color::Green),
        "system" => ("", Color::DarkGray),
        "tool" => ("Tool: ", Color::Yellow),
        "tool_result" => ("Result: ", Color::Blue),
        "error" => ("Error: ", Color::Red),
        _ => ("", Color::White),
    };

    let mut lines = vec![];
    for (i, line_text) in msg.content.lines().enumerate() {
        if i == 0 && !prefix.is_empty() {
            lines.push(Line::from(vec![
                Span::styled(prefix, Style::default().fg(color).add_modifier(ratatui::style::Modifier::BOLD)),
                Span::raw(line_text),
            ]));
        } else {
            let indent = " ".repeat(prefix.len());
            lines.push(Line::from(format!("{}{}", indent, line_text)));
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
        .title(format!(" Input  v{}", state.build_time))
        .border_style(if state.streaming {
            Style::default().fg(Color::DarkGray)
        } else {
            Style::default().fg(Color::White)
        });
    let paragraph = Paragraph::new(state.input.as_str()).block(block);
    f.render_widget(paragraph, area);
}

fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}
