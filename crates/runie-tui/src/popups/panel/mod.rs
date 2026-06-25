//! Panel dialog rendering — list and form views share the same popup shell.

use ratatui::{layout::Rect, style::Style, widgets::Paragraph, Frame};
use runie_core::Snapshot;

use crate::popups::{clear_panel_bg, palette_popup_rect};
use crate::theme::{block_popup, color_bg_panel};

mod form;
mod list;

pub fn panel_dialog(f: &mut Frame, snap: &Snapshot) {
    let dialog = match &snap.dialog {
        Some(d) => d,
        _ => return,
    };
    // Welcome screen has no panel stack
    if matches!(dialog, runie_core::commands::DialogState::Welcome) {
        crate::popups::welcome::render_welcome(f, snap);
        return;
    }
    let stack = match dialog.panel_stack() {
        Some(s) => s,
        None => return,
    };
    let panel = match stack.current() {
        Some(p) => p,
        None => return,
    };
    let root_closable = stack.root().map(|p| p.closable).unwrap_or(true);
    if panel.is_form() {
        form::render_form(f, panel, root_closable);
        return;
    }
    list::render_list(f, panel, stack.len() > 1, root_closable);
}

/// Clear/popup rect + block border + 1-cell inner margin.
pub(super) fn setup_popup(f: &mut Frame, title: &str) -> Rect {
    let popup_area = palette_popup_rect(f.area());
    clear_panel_bg(f, popup_area);
    let block = block_popup(title);
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

pub(super) fn hotkey_area(inner: &Rect) -> Rect {
    Rect {
        x: inner.x,
        y: inner.y + inner.height.saturating_sub(hotkey_area_height()),
        width: inner.width,
        height: hotkey_area_height(),
    }
}

pub(super) const fn hotkey_area_height() -> u16 {
    2
}

pub(super) struct ScrollLayout {
    pub area: Rect,
    pub bar_area: Rect,
    pub show_bar: bool,
    pub offset: usize,
    pub total: usize,
}

pub(super) fn compute_scrolling(
    area: &Rect,
    total: usize,
    selected: Option<usize>,
) -> ScrollLayout {
    let visible = area.height as usize;
    let show_bar = total > visible;
    let items_width = if show_bar {
        area.width.saturating_sub(1)
    } else {
        area.width
    };
    let offset = selected.map_or(0, |sel| {
        if total <= visible {
            0
        } else {
            sel.saturating_sub(visible / 2)
                .min(total.saturating_sub(visible))
        }
    });
    ScrollLayout {
        area: Rect {
            width: items_width,
            ..*area
        },
        bar_area: Rect {
            x: area.x + items_width,
            y: area.y,
            width: 1,
            height: area.height,
        },
        show_bar,
        offset,
        total,
    }
}

pub(super) fn style_border() -> Style {
    Style::default().fg(crate::theme::color_border())
}

pub(super) fn pad_to_width(s: &str, width: usize) -> String {
    let char_count = s.chars().count();
    if char_count >= width {
        s.to_owned()
    } else {
        format!("{}{}", s, " ".repeat(width - char_count))
    }
}
