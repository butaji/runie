use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Style, Color},
    text::{Line, Span},
    widgets::{Block, Widget},
};

use super::{InputBar, StyleHelpers};
use crate::theme::ThemeWrapper;

/// Standalone render function to avoid cloning InputBar fields in tui.rs
pub fn render_input_bar(
    lines: &[String],
    prompt: &str,
    right_info: &str,
    area: Rect,
    buf: &mut Buffer,
    theme: &ThemeWrapper,
) {
    let sp = StyleHelpers::new(theme);
    let border_color: Color = theme.color("border.unfocused").into();
    let blue_color: Color = theme.color("accent.primary").into();

    let block = Block::bordered()
        .border_style(Style::default().fg(border_color));
    let inner = block.inner(area);
    block.render(area, buf);

    render_content_lines(lines, prompt, inner, buf, &sp, blue_color);
    render_bottom_border(lines.len(), right_info, area, buf, border_color);
}

pub fn render_ref(input: &InputBar, area: Rect, buf: &mut Buffer, theme: &ThemeWrapper) {
    render_input_bar(
        input.lines.as_slice(),
        &input.prompt,
        &input.right_info,
        area,
        buf,
        theme,
    )
}

fn render_content_lines(
    lines: &[String],
    prompt: &str,
    area: Rect,
    buf: &mut Buffer,
    sp: &StyleHelpers,
    blue_color: Color,
) {
    let content_width = area.width.saturating_sub(4) as usize;

    for (line_idx, line_text) in lines.iter().enumerate() {
        let y = area.y + line_idx as u16;
        let x = area.x;

        if line_idx == 0 {
            render_first_content_line(x, y, line_text, prompt, buf, sp, content_width, blue_color);
        } else {
            render_subsequent_content_line(x, y, line_text, buf, sp, content_width);
        }
    }
}

fn render_first_content_line(
    x: u16, y: u16, line_text: &str, prompt: &str, buf: &mut Buffer,
    sp: &StyleHelpers, content_width: usize, blue_color: Color,
) {
    let prompt_line = Line::from(vec![
        Span::styled(prompt, Style::default().fg(blue_color)),
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

pub fn truncate_or_clone(text: &str, available: usize) -> std::borrow::Cow<'_, str> {
    let char_count = text.chars().count();
    if char_count > available {
        let truncated: String = text.chars().take(available.saturating_sub(3)).collect();
        std::borrow::Cow::Owned(format!("{}...", truncated))
    } else {
        std::borrow::Cow::Borrowed(text)
    }
}

fn render_bottom_border(
    line_count: usize,
    right_info: &str,
    area: Rect,
    buf: &mut Buffer,
    border_color: Color,
) {
    let bottom_y = area.y + 1 + line_count as u16;
    let info_text = if right_info.is_empty() {
        "model: claude-4"
    } else {
        right_info
    };
    let info_len = info_text.chars().count() as u16;
    // Layout: ╰ + dashes + " " + info_text + " " + "─╯"
    // Total fixed chars: 1 (╰) + 1 (space) + info_len + 1 (space) + 2 (─╯) = info_len + 5
    // dashes fill the space between ╰ and the info text
    let dash_count = area.width.saturating_sub(info_len + 5).max(0);
    let dash_str = "─".repeat(dash_count as usize);

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
