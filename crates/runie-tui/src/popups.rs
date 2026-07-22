//! Popup rendering — command palette, path suggestions, plan panel.
//!
//! Layout constants are centralized in `layout_constants.rs`.

use ratatui::{
    layout::Rect,
    prelude::Text,
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

pub mod layout_constants;
pub mod panel;
pub mod plan;
pub mod welcome;

/// Clear the given rect with the panel background color.
pub fn clear_panel_bg(f: &mut Frame, area: Rect) {
    f.render_widget(Clear, area);
    f.buffer_mut()
        .set_style(area, Style::default().bg(color_bg_panel()));
}

/// Compute the centered popup rect for the command palette.
pub fn palette_popup_rect(area: Rect) -> Rect {
    let popup_width = layout_constants::POPUP_WIDTH
        .min(area.width.saturating_sub(4))
        .max(layout_constants::POPUP_MIN_WIDTH);
    let popup_height = layout_constants::POPUP_HEIGHT
        .min(area.height.saturating_sub(4))
        .max(layout_constants::POPUP_MIN_HEIGHT);
    Rect {
        x: area.x + (area.width.saturating_sub(popup_width)) / 2,
        y: area.y + (area.height.saturating_sub(popup_height)) / 2,
        width: popup_width,
        height: popup_height,
    }
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

    let popup_rect = path_popup_area(f.area(), items.len());
    let lines = build_path_suggestion_lines(items, selected);
    let title = format!(" paths ({}) ", items.len());

    // setup_popup handles border + bg + 1-cell inner margin.
    clear_panel_bg(f, popup_rect);
    f.render_widget(Paragraph::new("").block(block_popup(&title)), popup_rect);

    let inner = Rect {
        x: popup_rect.x + 1,
        y: popup_rect.y + 1,
        width: popup_rect.width.saturating_sub(2),
        height: popup_rect.height.saturating_sub(2),
    };

    let content = Text::from(lines);
    f.render_widget(Paragraph::new(content), inner);
}

fn path_popup_area(area: Rect, item_count: usize) -> Rect {
    let display_count = item_count.min(layout_constants::PATH_DISPLAY_COUNT as usize) as u16;
    let max_height = display_count + layout_constants::PATH_POPUP_BORDER;
    Rect {
        x: area.x + 1,
        y: area.y + area.height.saturating_sub(4 + max_height),
        width: area
            .width
            .saturating_sub(2)
            .max(layout_constants::POPUP_MIN_WIDTH),
        height: max_height,
    }
}

fn build_path_suggestion_lines(items: &[runie_core::path_complete::PathCompletion], selected: usize) -> Vec<Line<'_>> {
    let mut lines: Vec<Line<'_>> = items
        .iter()
        .take(8)
        .enumerate()
        .map(|(i, item)| path_suggestion_line(item, i == selected))
        .collect();
    lines.push(Line::from(""));
    lines.push(Line::from("↑/↓=nav Enter=select Esc=close").style(style_hint()));
    lines
}

fn path_suggestion_line(item: &runie_core::path_complete::PathCompletion, is_selected: bool) -> Line<'_> {
    let prefix = if is_selected {
        GLYPH_SELECTED
    } else {
        GLYPH_UNSELECTED
    };
    let style = if is_selected {
        style_popup_selected()
    } else {
        style_popup_unselected()
    };
    let suffix = if item.is_dir { "/" } else { "" };
    Line::from(format!("{}{}{}", prefix, item.path, suffix)).style(style)
}
