//! Popup rendering — command palette, path suggestions, plan panel.
//!
//! Layout constants are centralized in `layout_constants.rs`.

use ratatui::{
    layout::Rect,
    style::Style,
    text::{Line, Text},
    widgets::{Clear, Paragraph},
    Frame,
};
use runie_core::Snapshot;

use crate::theme::{
    block_popup, color_bg_panel, style_hint, style_popup_selected, style_popup_unselected, GLYPH_SELECTED,
    GLYPH_UNSELECTED,
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
    // Degraded terminals must never receive a popup rectangle larger than the
    // frame. Grok keeps overlays inside the available cell grid during resize
    // storms; forcing the normal minimum dimensions here used to let the
    // border/content extend beyond very short or narrow frames.
    let popup_width = layout_constants::POPUP_WIDTH.min(area.width);
    let popup_height = layout_constants::POPUP_HEIGHT.min(area.height);
    Rect {
        x: area.x + (area.width.saturating_sub(popup_width)) / 2,
        y: area.y + (area.height.saturating_sub(popup_height)) / 2,
        width: popup_width,
        height: popup_height,
    }
}

#[cfg(test)]
mod popup_rect_tests {
    use super::palette_popup_rect;
    use ratatui::layout::Rect;

    #[test]
    fn popup_rect_is_contained_in_normal_frame() {
        let frame = Rect::new(0, 0, 100, 32);
        let popup = palette_popup_rect(frame);
        assert!(popup.x + popup.width <= frame.x + frame.width);
        assert!(popup.y + popup.height <= frame.y + frame.height);
    }

    #[test]
    fn popup_rect_never_exceeds_degraded_frame() {
        let frame = Rect::new(0, 0, 12, 4);
        let popup = palette_popup_rect(frame);
        assert_eq!(popup.width, 12);
        assert_eq!(popup.height, 4);
        assert!(popup.x + popup.width <= frame.x + frame.width);
        assert!(popup.y + popup.height <= frame.y + frame.height);
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

// ─────────────────────────────────────────────────────────────────────────────
// Inline slash-command dropdown (grok parity: slash_dropdown.rs)
// ─────────────────────────────────────────────────────────────────────────────

/// Render the inline slash-command dropdown anchored above the input box.
/// Rows: `❯ /name  desc` (selected) / `  /name  desc`. A bare match count
/// sits right-aligned in the top border. None when closed or empty.
pub fn slash_dropdown(f: &mut Frame, snap: &Snapshot) {
    let dd = match &snap.slash_dropdown {
        Some(dd) if !dd.matches.is_empty() => dd,
        _ => return,
    };
    let count = dd.matches.len().min(model_slash_max_rows());
    // Slash-triggered command selection is the same command palette as
    // Ctrl+P. Keep the shell identical; the match count belongs in the list,
    // not in a second window title variant.
    let inner = panel::setup_popup(f, " Commands ");
    let mut lines: Vec<Line<'static>> = dd
        .matches
        .iter()
        .take(count)
        .enumerate()
        .map(|(i, m)| {
            let selected = i == dd.selected;
            let prefix = if selected {
                GLYPH_SELECTED
            } else {
                GLYPH_UNSELECTED
            };
            let style = if selected {
                style_popup_selected()
            } else {
                style_popup_unselected()
            };
            Line::from(format!("{prefix}/{}  {}", m.name, m.desc)).style(style)
        })
        .collect();
    lines.push(Line::from(""));
    lines.push(Line::from("↑↓ navigate · enter select · esc close").style(style_hint()));
    f.render_widget(Paragraph::new(Text::from(lines)), inner);
}

/// Max visible dropdown rows (grok `MAX_VISIBLE_SUGGESTIONS`).
fn model_slash_max_rows() -> usize {
    6
}
