//! RenderPipe helper functions.

use ratatui::{buffer::Buffer, layout::Rect, style::Style, widgets::Widget};

use crate::theme::ThemeColors;

pub fn clear_background(buf: &mut Buffer, area: Rect, bg_color: ratatui::style::Color) {
    for y in area.y..area.y + area.height {
        for x in area.x..area.x + area.width {
            if let Some(cell) = buf.cell_mut((x, y)) {
                cell.set_char(' ');
                cell.set_style(Style::default().bg(bg_color));
            }
        }
    }
}

pub fn dim_background(buf: &mut Buffer, area: Rect, theme_colors: &ThemeColors) {
    let dim_color = match theme_colors.bg_base {
        ratatui::style::Color::Rgb(r, g, b) => {
            ratatui::style::Color::Rgb(
                (r as f32 * 0.5).round() as u8,
                (g as f32 * 0.5).round() as u8,
                (b as f32 * 0.5).round() as u8,
            )
        }
        ratatui::style::Color::Indexed(idx) => {
            ratatui::style::Color::Indexed(idx.saturating_sub(8))
        }
        _ => ratatui::style::Color::Black,
    };
    ratatui::widgets::Paragraph::new("")
        .style(Style::default().bg(dim_color))
        .render(area, buf);
}

pub fn blit_buffer(buf: &mut Buffer, area: Rect, src_area: Rect, src: &Buffer) {
    for y in 0..src.area.height {
        for x in 0..src.area.width {
            let cell = src.cell((x, y));
            let tx = src_area.x + x;
            let ty = src_area.y + y;
            if tx < area.width && ty < area.height {
                if let (Some(src_cell), Some(target)) = (cell, buf.cell_mut((tx, ty))) {
                    *target = src_cell.clone();
                }
            }
        }
    }
}
