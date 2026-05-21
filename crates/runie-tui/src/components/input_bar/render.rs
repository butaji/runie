use ratatui::{buffer::Buffer, layout::Rect, style::Style, text::{Line, Span}};

use super::{InputBar, StyleHelpers};
use crate::theme::ThemeWrapper;

pub fn render_ref(input: &InputBar, area: Rect, buf: &mut Buffer, theme: &ThemeWrapper) {
    let sp = StyleHelpers::new(theme);
    let border_color: ratatui::style::Color = theme.color("border.unfocused").into();
    let blue_color: ratatui::style::Color = theme.color("accent.primary").into();

    render_top_border(area, buf, border_color);
    render_content_lines(input, area, buf, &sp, border_color, blue_color);
    render_bottom_border(input, area, buf, border_color);
}

fn render_top_border(area: Rect, buf: &mut Buffer, border_color: ratatui::style::Color) {
    let style = Style::default().fg(border_color);
    buf.get_mut(area.x, area.y).set_char('╭').set_style(style);
    buf.get_mut(area.x + area.width - 1, area.y).set_char('╮').set_style(style);
    for x in 1..area.width.saturating_sub(1) {
        buf.get_mut(area.x + x, area.y).set_char('─').set_style(style);
    }
}

fn render_content_lines(
    input: &InputBar,
    area: Rect,
    buf: &mut Buffer,
    sp: &StyleHelpers,
    border_color: ratatui::style::Color,
    blue_color: ratatui::style::Color,
) {
    let border_style = Style::default().fg(border_color);
    let content_width = area.width.saturating_sub(4) as usize;

    for (line_idx, line_text) in input.lines.iter().enumerate() {
        let y = area.y + 1 + line_idx as u16;
        buf.get_mut(area.x, y).set_style(border_style).set_char('│');
        buf.get_mut(area.x + area.width - 1, y).set_style(border_style).set_char('│');

        let x = area.x + 1;
        if line_idx == 0 {
            render_first_content_line(x, y, line_text, buf, sp, content_width, blue_color);
        } else {
            render_subsequent_content_line(x, y, line_text, buf, sp, content_width);
        }
    }
}

fn render_first_content_line(
    x: u16, y: u16, line_text: &str, buf: &mut Buffer,
    sp: &StyleHelpers, content_width: usize, blue_color: ratatui::style::Color,
) {
    let prompt_style = Style::default().fg(blue_color);
    let prompt_line = Line::from(vec![
        Span::styled("❯", prompt_style),
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
    border_color: ratatui::style::Color,
) {
    let style = Style::default().fg(border_color);
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

    buf.get_mut(area.x, bottom_y).set_char('╰').set_style(style);
    for i in 0..dashes {
        let x = area.x + 1 + i;
        buf.get_mut(x, bottom_y).set_char('─').set_style(style);
    }

    let mut x = area.x + 1 + dashes;
    buf.get_mut(x, bottom_y).set_char(' ').set_style(style);
    x += 1;
    for ch in info_text.chars() {
        buf.get_mut(x, bottom_y).set_char(ch).set_style(style);
        x += 1;
    }
    buf.get_mut(x, bottom_y).set_char(' ').set_style(style);
    x += 1;
    buf.get_mut(x, bottom_y).set_char('─').set_style(style);
    x += 1;
    buf.get_mut(area.x + area.width - 1, bottom_y).set_char('╯').set_style(style);
}
