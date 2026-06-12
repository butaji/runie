use ratatui::{
    layout::Rect,
    style::Style,
    text::Line,
    widgets::{Clear, Paragraph},
    Frame,
};
use runie_core::Snapshot;

use crate::theme::{
    block_popup, color_bg_panel, style_hint, style_popup_selected, style_popup_unselected,
    GLYPH_SELECTED, GLYPH_UNSELECTED,
};

pub mod panel;

/// Clear the given rect with the panel background color.
pub fn clear_panel_bg(f: &mut Frame, area: Rect) {
    f.render_widget(Clear, area);
    f.buffer_mut()
        .set_style(area, Style::default().bg(color_bg_panel()));
}

pub fn path_suggestions(f: &mut Frame, snap: &Snapshot) {
    let items = match &snap.path_suggestions {
        Some(s) if !s.is_empty() => s,
        _ => return,
    };
    let selected = snap
        .path_selected
        .unwrap_or(0)
        .min(items.len().saturating_sub(1));
    let area = f.area();
    let display_count = items.len().min(8) as u16;
    let max_height = display_count + 4;
    let popup_area = Rect {
        x: area.x + 1,
        y: area.y + area.height.saturating_sub(4 + max_height),
        width: area.width.saturating_sub(2).max(20),
        height: max_height,
    };
    let mut lines: Vec<Line> = items
        .iter()
        .take(8)
        .enumerate()
        .map(|(i, item)| {
            let prefix = if i == selected {
                GLYPH_SELECTED
            } else {
                GLYPH_UNSELECTED
            };
            let style = if i == selected {
                style_popup_selected()
            } else {
                style_popup_unselected()
            };
            let suffix = if item.is_dir { "/" } else { "" };
            Line::from(format!("{}{}{}", prefix, item.path, suffix)).style(style)
        })
        .collect();
    lines.push(Line::from(""));
    lines.push(Line::from("↑/↓=nav Enter=select Esc=close").style(style_hint()));
    clear_panel_bg(f, popup_area);
    f.render_widget(
        Paragraph::new(lines)
            .style(Style::default().bg(color_bg_panel()))
            .block(block_popup(&format!(" paths ({}) ", items.len()))),
        popup_area,
    );
}

pub fn palette_popup_rect(area: Rect) -> Rect {
    let popup_width = 60u16.min(area.width.saturating_sub(4)).max(20);
    let popup_height = 18u16.min(area.height.saturating_sub(4)).max(6);
    Rect {
        x: area.x + (area.width.saturating_sub(popup_width)) / 2,
        y: area.y + (area.height.saturating_sub(popup_height)) / 2,
        width: popup_width,
        height: popup_height,
    }
}
