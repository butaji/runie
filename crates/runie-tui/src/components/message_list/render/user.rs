use ratatui::{buffer::Buffer, layout::Rect, style::Style};

use crate::components::message_list::WrapCache;
use crate::theme::ThemeWrapper;

/// Render a user message
pub fn render_user_msg(
    text: &str,
    area: Rect,
    row: u16,
    margin_x: u16,
    _text_x: u16,
    _max_rows: u16,
    buf: &mut Buffer,
    theme: &ThemeWrapper,
    accent_primary: ratatui::style::Color,
    wrap_cache: &mut WrapCache,
) -> u16 {
    let width = (area.width - margin_x + area.x - 2) as usize;
    let content_width = area.width - margin_x + area.x - 4;

    // Prefix with accent color
    if let Some(cell) = buf.cell_mut((margin_x, area.y + row)) {
        cell.set_char('▸');
        cell.set_style(Style::default().fg(accent_primary));
    }

    // Wrap user text
    let wrapped = wrap_cache.get_wrapped(text.trim(), content_width as usize);
    let mut rendered = 0u16;

    for (i, line_text) in wrapped.iter().enumerate() {
        let line_y = row + i as u16;
        if line_y >= area.height {
            break;
        }
        let line = ratatui::text::Line::raw(line_text.as_str()).style(Style::default().fg(accent_primary));
        buf.set_line(margin_x + 2, area.y + line_y, &line, content_width);
        rendered += 1;
    }

    rendered.max(1)
}
