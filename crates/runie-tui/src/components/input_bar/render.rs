use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Style, Color},
    text::{Line, Span},
    widgets::{Block, Widget},
};

use super::{InputBar, StyleHelpers};
use crate::theme::ThemeWrapper;

pub fn render_ref(input: &InputBar, area: Rect, buf: &mut Buffer, theme: &ThemeWrapper) {
    let sp = StyleHelpers::new(theme);
    let border_color: Color = theme.color("border.unfocused").into();
    let bg: Color = theme.color("bg.primary").into();
    let blue_color: Color = theme.color("accent.primary").into();

    let block = Block::bordered()
        .border_style(Style::default().fg(border_color))
        .style(Style::default().bg(bg));
    let inner = block.inner(area);
    block.render(area, buf);

    render_content_lines(input, inner, buf, &sp, blue_color);
    render_bottom_border(input, area, buf, border_color);
}

fn render_content_lines(
    input: &InputBar,
    area: Rect,
    buf: &mut Buffer,
    sp: &StyleHelpers,
    blue_color: Color,
) {
    let content_width = area.width.saturating_sub(4) as usize;

    for (line_idx, line_text) in input.lines.iter().enumerate() {
        let y = area.y + line_idx as u16;
        let x = area.x;

        if line_idx == 0 {
            render_first_content_line(x, y, line_text, buf, sp, content_width, blue_color);
        } else {
            render_subsequent_content_line(x, y, line_text, buf, sp, content_width);
        }
    }
}

fn render_first_content_line(
    x: u16, y: u16, line_text: &str, buf: &mut Buffer,
    sp: &StyleHelpers, content_width: usize, blue_color: Color,
) {
    let prompt_line = Line::from(vec![
        Span::styled("❯", Style::default().fg(blue_color)),
        Span::styled(" ", Style::default()),
    ]);
    buf.set_line(x, y, &prompt_line, 2);
    let text_x = x + 2;
    let available = content_width.saturating_sub(2);
    let display_text = truncate_or_clone(line_text, available);
    let text_line = Line::from(vec![Span::styled(display_text, sp.primary())]);
    buf.set_line(text_x, y, &text_line, available as u16);
}

fn render_subsequent_content_line(
    x: u16, y: u16, line_text: &str, buf: &mut Buffer,
    sp: &StyleHelpers, content_width: usize,
) {
    let indent_line = Line::from(vec![Span::styled("  ", sp.dim())]);
    buf.set_line(x, y, &indent_line, 2);
    let text_x = x + 2;
    let available = content_width.saturating_sub(2);
    let display_text = truncate_or_clone(line_text, available);
    let text_line = Line::from(vec![Span::styled(display_text, sp.primary())]);
    buf.set_line(text_x, y, &text_line, available as u16);
}

fn truncate_or_clone(text: &str, available: usize) -> String {
    if text.len() > available {
        format!("{}...", &text[..available.saturating_sub(3)])
    } else {
        text.to_string()
    }
}

fn render_bottom_border(
    input: &InputBar,
    area: Rect,
    buf: &mut Buffer,
    border_color: Color,
) {
    let bottom_y = area.y + 1 + input.lines.len() as u16;
    let info_text = if input.right_info.is_empty() {
        "model: claude-4"
    } else {
        &input.right_info
    };
    let info_len = info_text.len() as u16;
    let inner_width = area.width.saturating_sub(2);
    let fixed_width = info_len + 5;
    let dashes = inner_width.saturating_sub(fixed_width).max(0);

    let dash_str = "─".repeat(dashes as usize);
    let bottom_line = Line::from(vec![
        Span::styled("╰", Style::default().fg(border_color)),
        Span::styled(&dash_str, Style::default().fg(border_color)),
        Span::styled(" ", Style::default().fg(border_color)),
        Span::styled(info_text, Style::default().fg(border_color)),
        Span::styled(" ", Style::default().fg(border_color)),
        Span::styled("─", Style::default().fg(border_color)),
        Span::styled("╯", Style::default().fg(border_color)),
    ]);

    buf.set_line(area.x, bottom_y, &bottom_line, area.width);
}
