use ratatui::{buffer::Buffer, layout::Rect, style::Style};

use crate::components::message_list::WrapCache;
use crate::glyphs;
use crate::theme::ThemeWrapper;

/// Render a user message
pub fn render_user_msg(
    text: &str,
    timestamp: Option<&str>,
    area: Rect,
    row: u16,
    margin_x: u16,
    _text_x: u16,
    _max_rows: u16,
    buf: &mut Buffer,
    theme: &ThemeWrapper,
    wrap_cache: &mut WrapCache,
) -> u16 {
    let bg_color: ratatui::style::Color = theme.color("border.unfocused").into();
    let chevron_color: ratatui::style::Color = theme.color("accent.primary").into();
    let text_primary: ratatui::style::Color = theme.color("text.primary").into();
    // Account for 1-symbol horizontal padding on each side + chevron + space
    let text_width = (area.width - margin_x + area.x - 6) as usize;

    // Wrap user text
    let wrapped = wrap_cache.get_wrapped(text.trim(), text_width);

    let content_lines = if wrapped.is_empty() { 1 } else { wrapped.len() };
    let total_height = content_lines + 2; // +2 for vertical padding (1 above, 1 below)

    // Render vertical padding ABOVE (1 line)
    let content_start_y = area.y + row + 1;
    if content_start_y > area.y {
        let padding_y = content_start_y - 1;
        for x in (margin_x as usize)..(area.right() as usize) {
            if let Some(cell) = buf.cell_mut((x as u16, padding_y)) {
                cell.set_char(' ');
                cell.set_style(Style::default().bg(bg_color));
            }
        }
    }

    // Render content lines with horizontal padding
    for (i, line_text) in wrapped.iter().enumerate() {
        let line_y = content_start_y + i as u16;
        if line_y >= area.bottom() {
            break;
        }
        // Fill full-width gray background for the line
        for x in (margin_x as usize)..(area.right() as usize) {
            if let Some(cell) = buf.cell_mut((x as u16, line_y)) {
                cell.set_char(' ');
                cell.set_style(Style::default().bg(bg_color));
            }
        }

        if i == 0 {
            // First line: chevron aligned with input box prompt
            let chevron_x = margin_x;
            if let Some(cell) = buf.cell_mut((chevron_x, line_y)) {
                cell.set_char(glyphs::CHEVRON);
                cell.set_style(Style::default().fg(chevron_color).bg(bg_color));
            }
            let text_x = chevron_x + 2;
            let first_line = ratatui::text::Line::raw(line_text.as_str())
                .style(Style::default().fg(text_primary).bg(bg_color));
            buf.set_line(text_x, line_y, &first_line, text_width as u16);

            // Timestamp on first line for single-line messages
            if let Some(ts) = timestamp {
                if wrapped.len() <= 1 {
                    let ts_len = ts.len() as u16;
                    let ts_x = area.right().saturating_sub(ts_len + 1);
                    if ts_x > text_x {
                        let ts_color: ratatui::style::Color = theme.color("text.muted").into();
                        let ts_line = ratatui::text::Line::raw(ts)
                            .style(Style::default().fg(ts_color).bg(bg_color));
                        buf.set_line(ts_x, line_y, &ts_line, ts_len);
                    }
                }
            }
        } else {
            // Continuation lines: aligned with text after chevron
            let text_x = margin_x + 2;
            let line = ratatui::text::Line::raw(line_text.as_str())
                .style(Style::default().fg(text_primary).bg(bg_color));
            buf.set_line(text_x, line_y, &line, text_width as u16);
        }
    }

    // Timestamp on last line for multi-line messages
    if let Some(ts) = timestamp {
        if wrapped.len() > 1 {
            let last_line_y = content_start_y + wrapped.len().saturating_sub(1) as u16;
            let ts_len = ts.len() as u16;
            let ts_x = area.right().saturating_sub(ts_len + 2); // 1-symbol right padding
            if ts_x > margin_x + 1 {
                let ts_color: ratatui::style::Color = theme.color("text.muted").into();
                let ts_line = ratatui::text::Line::raw(ts)
                    .style(Style::default().fg(ts_color).bg(bg_color));
                buf.set_line(ts_x, last_line_y, &ts_line, ts_len);
            }
        }
    }

    // Render vertical padding BELOW (1 line)
    let padding_below_y = content_start_y + content_lines as u16;
    if padding_below_y < area.bottom() {
        for x in (margin_x as usize)..(area.right() as usize) {
            if let Some(cell) = buf.cell_mut((x as u16, padding_below_y)) {
                cell.set_char(' ');
                cell.set_style(Style::default().bg(bg_color));
            }
        }
    }

    total_height as u16
}
