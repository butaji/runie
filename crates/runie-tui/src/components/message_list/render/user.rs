use ratatui::{buffer::Buffer, layout::Rect, style::Style};

use crate::components::message_list::WrapCache;
use crate::theme::ThemeWrapper;

/// Render a user message
pub fn render_user_msg(
    text: &str,
    timestamp: Option<String>,
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
    let chevron_color: ratatui::style::Color = theme.color("accent.primary").into(); // Match input box prompt
    let text_primary: ratatui::style::Color = theme.color("text.primary").into();
    let text_width = (area.width - margin_x + area.x - 2) as usize;

    // Wrap user text (accounting for chevron + space on first line)
    let wrapped = wrap_cache.get_wrapped(text.trim(), text_width.saturating_sub(3));

    // For multi-line content, timestamp goes on the LAST line of content
    // For single-line, timestamp goes on that same line
    let text_lines = if wrapped.len() == 1 { 1 } else { wrapped.len() };
    let timestamp_line_offset = if wrapped.len() <= 1 { 0 } else { wrapped.len() - 1 };

    // Render first line: chevron + text
    let first_line_y = area.y + row;

    // Render 1 padding line ABOVE the content
    if first_line_y > area.y {
        let padding_above_y = first_line_y - 1;
        for x in (margin_x as usize)..(area.right() as usize) {
            if let Some(cell) = buf.cell_mut((x as u16, padding_above_y)) {
                cell.set_char(' ');
                cell.set_style(Style::default().bg(bg_color));
            }
        }
    }

    if first_line_y < area.bottom() {
        // Set full-width background for the line
        for x in (margin_x as usize)..(area.right() as usize) {
            if let Some(cell) = buf.cell_mut((x as u16, first_line_y)) {
                cell.set_char(' ');
                cell.set_style(Style::default().bg(bg_color));
            }
        }

        // Chevron - uses same color as input box border (border.unfocused)
        if let Some(cell) = buf.cell_mut((margin_x, first_line_y)) {
            cell.set_char('\u{203A}');
            cell.set_style(Style::default().fg(chevron_color).bg(bg_color));
        }

        // First line of wrapped text starts after chevron + space
        let first_line_text = wrapped.first().map(|s| s.as_str()).unwrap_or("");
        let first_line = ratatui::text::Line::raw(first_line_text)
            .style(Style::default().fg(text_primary).bg(bg_color));
        buf.set_line(margin_x + 2, first_line_y, &first_line, text_width.saturating_sub(2) as u16);

        // Timestamp on first line for single-line messages
        if timestamp.is_some() && wrapped.len() <= 1 {
            if let Some(ts) = timestamp.as_ref() {
                let ts_len = ts.len() as u16;
                let ts_x = area.right().saturating_sub(ts_len + 1);
                if ts_x > margin_x + 2 {
                    let ts_color: ratatui::style::Color = theme.color("text.muted").into();
                    let ts_line = ratatui::text::Line::raw(ts.as_str())
                        .style(Style::default().fg(ts_color).bg(bg_color));
                    buf.set_line(ts_x, first_line_y, &ts_line, ts_len);
                }
            }
        }
    }

    // Render remaining wrapped lines (continuation lines, no chevron)
    for (i, line_text) in wrapped.iter().enumerate().skip(1) {
        let cont_line_y = row + i as u16;
        if cont_line_y >= area.height || area.y + cont_line_y >= area.bottom() {
            break;
        }
        // Set full-width background
        for x in (margin_x as usize)..(area.right() as usize) {
            if let Some(cell) = buf.cell_mut((x as u16, area.y + cont_line_y)) {
                cell.set_char(' ');
                cell.set_style(Style::default().bg(bg_color));
            }
        }
        let line = ratatui::text::Line::raw(line_text.as_str())
            .style(Style::default().fg(text_primary).bg(bg_color));
        buf.set_line(margin_x, area.y + cont_line_y, &line, text_width as u16);
    }

    // Timestamp on last line for multi-line messages
    if timestamp.is_some() && wrapped.len() > 1 {
        let last_line_y = area.y + row + timestamp_line_offset as u16;
        if let Some(ts) = timestamp.as_ref() {
            let ts_len = ts.len() as u16;
            let ts_x = area.right().saturating_sub(ts_len + 1);
            if ts_x > margin_x {
                let ts_color: ratatui::style::Color = theme.color("text.muted").into();
                let ts_line = ratatui::text::Line::raw(ts.as_str())
                    .style(Style::default().fg(ts_color).bg(bg_color));
                buf.set_line(ts_x, last_line_y, &ts_line, ts_len);
            }
        }
    }

    // Render 2 padding lines BELOW the content
    let last_content_line_y = area.y + row + text_lines as u16 - 1;
    for pad_offset in 1..=2 {
        let padding_below_y = last_content_line_y + pad_offset;
        if padding_below_y < area.bottom() {
            for x in (margin_x as usize)..(area.right() as usize) {
                if let Some(cell) = buf.cell_mut((x as u16, padding_below_y)) {
                    cell.set_char(' ');
                    cell.set_style(Style::default().bg(bg_color));
                }
            }
        }
    }

    text_lines as u16
}
