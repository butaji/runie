//! Panel dialog rendering — list and form views share the same popup shell.

use ratatui::{
    layout::Rect,
    style::Style,
    text::Line,
    widgets::{Block, Borders, Paragraph},
    Frame,
};
use runie_core::dialog::Panel;
use runie_core::Snapshot;

use crate::popups::{clear_panel_bg, palette_popup_rect};
use crate::theme::{block_popup, color_bg_panel};

mod form;
mod list;

pub fn panel_dialog(f: &mut Frame, snap: &Snapshot) {
    let stack = match snap.dialog.as_ref().and_then(|d| d.panel_stack()) {
        Some(s) => s,
        _ => return,
    };
    let panel = match stack.current() {
        Some(p) => p,
        None => return,
    };
    if panel.is_form() {
        form::render_form(f, panel);
        return;
    }
    list::render_list(f, panel, stack.len() > 1);
}

/// Clear/popup rect + block border + 1-cell inner margin.
pub(super) fn setup_popup(f: &mut Frame, title: &str) -> Rect {
    let popup_area = palette_popup_rect(f.area());
    clear_panel_bg(f, popup_area);
    let title_owned = format!(" {} ", title);
    let block = block_popup(&title_owned);
    let inner = block.inner(popup_area);
    f.render_widget(Paragraph::new("").block(block), popup_area);
    f.buffer_mut()
        .set_style(inner, Style::default().bg(color_bg_panel()));
    // 1-symbol / 1-line empty margin on all sides
    Rect {
        x: inner.x + 1,
        y: inner.y + 1,
        width: inner.width.saturating_sub(2),
        height: inner.height.saturating_sub(2),
    }
}

pub(super) fn below(area: &Rect, header_h: u16) -> Rect {
    Rect {
        x: area.x,
        y: area.y + header_h,
        height: area.height.saturating_sub(header_h),
        width: area.width,
    }
}

pub(super) fn hotkey_area(inner: &Rect) -> Rect {
    Rect {
        x: inner.x,
        y: inner.y + inner.height.saturating_sub(2),
        width: inner.width,
        height: 2,
    }
}

pub(super) fn compute_scrolling(
    area: &Rect,
    total: usize,
    selected: Option<usize>,
) -> (Rect, Rect, bool, usize) {
    let visible = area.height as usize;
    let show_scrollbar = total > visible;
    let items_width = if show_scrollbar {
        area.width.saturating_sub(1)
    } else {
        area.width
    };
    let scroll_area = Rect {
        width: items_width,
        ..*area
    };
    let scrollbar_area = Rect {
        x: area.x + items_width,
        y: area.y,
        width: 1,
        height: area.height,
    };
    let scroll_offset = if let Some(sel) = selected {
        if total <= visible {
            0
        } else {
            sel.saturating_sub(visible / 2)
                .min(total.saturating_sub(visible))
        }
    } else {
        0
    };
    (scroll_area, scrollbar_area, show_scrollbar, scroll_offset)
}

pub(super) fn style_border() -> Style {
    Style::default().fg(crate::theme::color_border())
}

pub(super) fn circled_number(n: usize) -> String {
    match n {
        1 => "①".into(),
        2 => "②".into(),
        3 => "③".into(),
        4 => "④".into(),
        5 => "⑤".into(),
        6 => "⑥".into(),
        7 => "⑦".into(),
        8 => "⑧".into(),
        9 => "⑨".into(),
        _ => format!("{}.", n),
    }
}

pub(super) fn truncate(s: &str, max: usize) -> (String, bool) {
    if s.chars().count() <= max {
        (s.to_string(), false)
    } else {
        let truncated: String = s.chars().take(max.saturating_sub(1)).collect();
        (truncated, true)
    }
}

pub(super) fn pad_to_width(s: &str, width: usize) -> String {
    let char_count = s.chars().count();
    if char_count >= width {
        s.to_string()
    } else {
        format!("{}{}", s, " ".repeat(width - char_count))
    }
}
