use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Modifier, Style},
    text::Line,
};
use crate::theme::ThemeWrapper;

#[derive(Debug, Clone)]
pub struct ThinkingBlock {
    pub content: String,
    pub duration_secs: f64,
    pub collapsed: bool,
    pub animation_frame: usize,
}

pub fn render_thinking_block(
    block: &ThinkingBlock,
    area: Rect,
    buf: &mut Buffer,
    theme: &ThemeWrapper,
    row: u16,
    margin_x: u16,
) -> u16 {
    let accent_color: Color = theme.color("accent.thinking").into();
    let text_color: Color = theme.color("text.primary").into();

    let y = area.y + row;
    if y >= area.bottom() {
        return 0;
    }

    if block.collapsed {
        // Collapsed: "┃  ◆ Thinking…"
        let header_text = "┃  ◆ Thinking…";
        let header = Line::styled(header_text, Style::default().fg(accent_color).add_modifier(Modifier::BOLD));
        buf.set_line(margin_x, y, &header, area.width.saturating_sub(margin_x));
        return 1;
    }

    // Expanded: "┃  ◆ Thinking…" followed by content lines with "┃  " prefix
    let mut rendered = 0u16;

    // Header line: "┃  ◆ Thinking…"
    if y + rendered < area.bottom() {
        let line_y = y + rendered;
        let header_text = format!("┃  ◆ Thinking…");
        let header = Line::styled(header_text, Style::default().fg(accent_color).add_modifier(Modifier::BOLD));
        buf.set_line(margin_x, line_y, &header, area.width.saturating_sub(margin_x));
        rendered += 1;
    }

    // Content lines: "┃  <content>"
    let content = block.content.trim();
    let lines: Vec<&str> = content.lines().collect();

    for line_text in lines.iter() {
        if y + rendered >= area.bottom() {
            break;
        }
        let line_y = y + rendered;
        let content_line = format!("┃  {}", line_text);
        let line = Line::styled(content_line, Style::default().fg(text_color));
        buf.set_line(margin_x, line_y, &line, area.width.saturating_sub(margin_x));
        rendered += 1;
    }

    rendered as u16
}

pub fn render_thought_indicator(
    _duration_secs: f64,
    area: Rect,
    buf: &mut Buffer,
    theme: &ThemeWrapper,
    row: u16,
    margin_x: u16,
) -> u16 {
    let accent_color: Color = theme.color("accent.thinking").into();
    let y = area.y + row;
    let text = format!("┃  ◆ Thinking…");
    let line = Line::styled(text, Style::default().fg(accent_color));
    buf.set_line(margin_x, y, &line, area.width.saturating_sub(margin_x));
    1
}