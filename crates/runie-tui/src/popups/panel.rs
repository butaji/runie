use ratatui::{
    layout::Rect,
    style::Style,
    text::Line,
    widgets::Paragraph,
    Frame,
};
use runie_core::Snapshot;

use crate::theme::{
    GLYPH_SELECTED, GLYPH_UNSELECTED, block_popup, style_popup_selected,
    style_popup_unselected, style_hint, style_thinking, style_user,
    color_bg_panel,
};
use crate::ui::{parse_hint_spans, render_scrollbar};
use crate::popups::{palette_popup_rect, popup_p, clear_panel_bg};

pub fn panel_dialog(f: &mut Frame, snap: &Snapshot) {
    let stack = match &snap.dialog {
        Some(runie_core::commands::DialogState::PanelStack(stack)) => stack,
        _ => return,
    };
    let panel = match stack.current() {
        Some(p) => p,
        None => return,
    };

    let popup_area = palette_popup_rect(f.area());
    clear_panel_bg(f, popup_area);
    let title = if stack.len() > 1 {
        format!(" {} › {} ", stack.panels[0].title, panel.title)
    } else {
        format!(" {} ", panel.title)
    };
    let block = block_popup(&title);
    let inner = block.inner(popup_area);
    f.render_widget(Paragraph::new("").block(block), popup_area);
    f.buffer_mut().set_style(inner, Style::default().bg(color_bg_panel()));

    let content_height = inner.height.saturating_sub(2);
    let hotkeys_area = Rect {
        x: inner.x,
        y: inner.y + content_height,
        width: inner.width,
        height: 2,
    };
    let content_area = Rect {
        height: content_height,
        ..inner
    };

    let mut header_lines: Vec<Line> = Vec::new();
    if panel.filterable {
        header_lines.push(Line::from(format!("❯ {}", panel.filter)).style(style_user()));
    }
    let sep_width = inner.width as usize;
    header_lines.push(Line::from("─".repeat(sep_width)).style(style_hint()));
    let header_height = header_lines.len() as u16;

    let header_area = Rect {
        height: header_height,
        ..content_area
    };
    let items_area = Rect {
        x: content_area.x,
        y: content_area.y + header_height,
        height: content_height.saturating_sub(header_height),
        width: content_area.width,
    };

    let filtered = panel.filtered_items();
    let mut item_lines: Vec<Line> = Vec::new();
    let mut selected_line: Option<usize> = None;
    let mut nav_idx = 0;

    for item in filtered.iter() {
        match item {
            runie_core::dialog::PanelItem::Header(text) => {
                item_lines.push(Line::from(format!("  {}", text)).style(style_thinking()));
            }
            runie_core::dialog::PanelItem::Separator => {
                item_lines.push(Line::from(""));
            }
            runie_core::dialog::PanelItem::Action { label, .. } => {
                if nav_idx == panel.selected {
                    selected_line = Some(item_lines.len());
                }
                let prefix = if nav_idx == panel.selected { GLYPH_SELECTED } else { GLYPH_UNSELECTED };
                let style = if nav_idx == panel.selected { style_popup_selected() } else { style_popup_unselected() };
                item_lines.push(Line::from(format!("{}{}", prefix, label)).style(style));
                nav_idx += 1;
            }
            runie_core::dialog::PanelItem::Toggle { label, value, .. } => {
                if nav_idx == panel.selected {
                    selected_line = Some(item_lines.len());
                }
                let prefix = if nav_idx == panel.selected { GLYPH_SELECTED } else { GLYPH_UNSELECTED };
                let style = if nav_idx == panel.selected { style_popup_selected() } else { style_popup_unselected() };
                let mark = if *value { "[x]" } else { "[ ]" };
                item_lines.push(Line::from(format!("{}{} {}", prefix, mark, label)).style(style));
                nav_idx += 1;
            }
            runie_core::dialog::PanelItem::Select { label, current, .. } => {
                if nav_idx == panel.selected {
                    selected_line = Some(item_lines.len());
                }
                let prefix = if nav_idx == panel.selected { GLYPH_SELECTED } else { GLYPH_UNSELECTED };
                let style = if nav_idx == panel.selected { style_popup_selected() } else { style_popup_unselected() };
                item_lines.push(Line::from(format!("{}{:20} {}", prefix, label, current)).style(style));
                nav_idx += 1;
            }
        }
    }

    let total_item_lines = item_lines.len();
    let visible_items_height = items_area.height as usize;

    let scroll_offset = if let Some(sel) = selected_line {
        if total_item_lines <= visible_items_height {
            0
        } else {
            sel.saturating_sub(visible_items_height / 2)
                .min(total_item_lines.saturating_sub(visible_items_height))
        }
    } else {
        0
    };

    let show_scrollbar = total_item_lines > visible_items_height;
    let items_width = if show_scrollbar {
        items_area.width.saturating_sub(1)
    } else {
        items_area.width
    };
    let scroll_area = Rect { width: items_width, ..items_area };
    let scrollbar_area = Rect {
        x: items_area.x + items_width,
        y: items_area.y,
        width: 1,
        height: items_area.height,
    };

    f.render_widget(popup_p(header_lines), header_area);
    f.render_widget(
        popup_p(item_lines).scroll((scroll_offset as u16, 0)),
        scroll_area,
    );

    if show_scrollbar {
        render_scrollbar(f, scrollbar_area, total_item_lines, scroll_offset as u16, visible_items_height);
    }

    let hotkey_text = if stack.len() > 1 {
        "↑↓ navigate · enter select · ← back · esc close"
    } else {
        "↑↓ navigate · enter select · esc close"
    };
    let hint_lines = vec![
        Line::from(""),
        Line::from(parse_hint_spans(hotkey_text)),
    ];
    f.render_widget(popup_p(hint_lines), hotkeys_area);
}
