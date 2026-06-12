//! List-style panel rendering (command palette, settings, model selector, etc.)

use ratatui::{
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Paragraph, Wrap},
    Frame,
};
use runie_core::dialog::{Panel, PanelItem};

use crate::theme::{
    color_accent, color_bg, color_bg_panel, color_dim, style_hint, style_popup_unselected,
    style_thinking, style_user, GLYPH_SELECTED, GLYPH_UNSELECTED,
};
use crate::ui::{parse_hint_spans, render_scrollbar};

use super::{below, compute_scrolling, hotkey_area, setup_popup};

pub(super) fn render_list(f: &mut Frame, panel: &Panel, show_breadcrumb: bool) {
    let inner = setup_popup(f, &panel.title);
    let inner_w = inner.width as usize;
    let (header_lines, header_height) = build_header(panel, inner_w);
    let header_area = Rect {
        height: header_height,
        ..inner
    };
    let items_area = below(&inner, header_height);
    let total = panel.filtered_items().len();
    let visible = items_area.height as usize;
    let show_scrollbar = total > visible;
    let item_width = if show_scrollbar {
        inner_w.saturating_sub(1)
    } else {
        inner_w
    };
    let (item_lines, selected_line) = build_items(panel, item_width);
    let (scroll_area, scrollbar_area, _, scroll_offset) =
        compute_scrolling(&items_area, item_lines.len(), selected_line);
    let bg = Style::default().bg(color_bg_panel());
    f.render_widget(Paragraph::new(header_lines).style(bg), header_area);
    f.render_widget(
        Paragraph::new(item_lines)
            .style(bg)
            .wrap(Wrap { trim: false })
            .scroll((scroll_offset as u16, 0)),
        scroll_area,
    );
    if show_scrollbar {
        render_scrollbar(
            f,
            scrollbar_area,
            total,
            scroll_offset as u16,
            scroll_area.height as usize,
        );
    }
    let hotkey_text = if show_breadcrumb {
        "↑↓ navigate · enter select · ← back · esc close"
    } else {
        "↑↓ navigate · enter select · esc close"
    };
    f.render_widget(
        Paragraph::new(vec![
            Line::from(""),
            Line::from(parse_hint_spans(hotkey_text)),
        ])
        .style(bg),
        hotkey_area(&inner),
    );
}

fn build_header(panel: &Panel, inner_w: usize) -> (Vec<Line<'_>>, u16) {
    let mut lines: Vec<Line> = Vec::new();
    if panel.filterable {
        lines.push(Line::from(format!("❯ {}", panel.filter)).style(style_user()));
    }
    lines.push(Line::from("─".repeat(inner_w)).style(style_hint()));
    let h = lines.len() as u16;
    (lines, h)
}

fn build_items(panel: &Panel, width: usize) -> (Vec<Line<'_>>, Option<usize>) {
    let filtered = panel.filtered_items();
    let mut lines: Vec<Line> = Vec::new();
    let mut selected_line: Option<usize> = None;
    let mut nav_idx = 0;

    for item in filtered.iter() {
        match item {
            PanelItem::Header(_) | PanelItem::Separator => push_static_item(&mut lines, item),
            PanelItem::FormField { .. } | PanelItem::FormSubmit => {}
            other => {
                if nav_idx == panel.selected {
                    selected_line = Some(lines.len());
                }
                push_navigable_item(&mut lines, other, nav_idx == panel.selected, width);
                nav_idx += 1;
            }
        }
    }
    (lines, selected_line)
}

fn push_static_item(lines: &mut Vec<Line>, item: &PanelItem) {
    match item {
        PanelItem::Header(text) => {
            lines.push(Line::from(format!("  {}", text)).style(style_thinking()));
        }
        PanelItem::Separator => lines.push(Line::from("")),
        _ => {}
    }
}

fn push_navigable_item<'a>(
    lines: &mut Vec<Line<'a>>,
    item: &'a PanelItem,
    selected: bool,
    width: usize,
) {
    match item {
        PanelItem::Action { label, .. } => push_action(lines, label, selected, width),
        PanelItem::Toggle { label, value, .. } => {
            push_toggle(lines, label, *value, selected, width)
        }
        PanelItem::Select { label, current, .. } => {
            push_select(lines, label, current, selected, width)
        }
        _ => {}
    }
}

/// Split a label like "readonly Toggle read-only mode" into ("readonly", " Toggle read-only mode").
fn split_label(label: &str) -> (&str, &str) {
    match label.find(' ') {
        Some(i) => (&label[..i], &label[i..]),
        None => (label, ""),
    }
}

fn push_action<'a>(lines: &mut Vec<Line<'a>>, label: &'a str, selected: bool, width: usize) {
    let prefix = if selected {
        GLYPH_SELECTED
    } else {
        GLYPH_UNSELECTED
    };
    let (name, desc) = split_label(label);
    if selected {
        let bg = color_accent();
        let fg = color_bg();
        // Description is also inverted (dark on accent) but uses the panel
        // background color so it reads lower-contrast than the command name.
        let desc_fg = color_bg_panel();
        let prefix_st = Style::default().bg(bg).fg(fg).add_modifier(Modifier::BOLD);
        let name_st = Style::default().bg(bg).fg(fg).add_modifier(Modifier::BOLD);
        let desc_st = Style::default().bg(bg).fg(desc_fg);
        let mut spans = vec![Span::styled(prefix, prefix_st), Span::styled(name, name_st)];
        if !desc.is_empty() {
            spans.push(Span::styled(desc, desc_st));
        }
        pad_selected_line(&mut spans, width, bg);
        lines.push(Line::from(spans));
    } else {
        let name_st = style_popup_unselected().add_modifier(Modifier::BOLD);
        let desc_st = Style::default().fg(color_dim());
        let mut spans = vec![
            Span::styled(prefix, style_popup_unselected()),
            Span::styled(name, name_st),
        ];
        if !desc.is_empty() {
            spans.push(Span::styled(desc, desc_st));
        }
        lines.push(Line::from(spans));
    }
}

fn push_toggle(lines: &mut Vec<Line>, label: &str, value: bool, selected: bool, width: usize) {
    let prefix = if selected {
        GLYPH_SELECTED
    } else {
        GLYPH_UNSELECTED
    };
    let mark = if value { "[x]" } else { "[ ]" };
    if selected {
        let bg = color_accent();
        let fg = color_bg();
        let st = Style::default().bg(bg).fg(fg).add_modifier(Modifier::BOLD);
        let mut spans = vec![
            Span::styled(prefix, st),
            Span::styled(format!("{} {}", mark, label), st),
        ];
        pad_selected_line(&mut spans, width, bg);
        lines.push(Line::from(spans));
    } else {
        let st = style_popup_unselected();
        lines.push(Line::from(format!("{}{} {}", prefix, mark, label)).style(st));
    }
}

fn push_select<'a>(
    lines: &mut Vec<Line<'a>>,
    label: &'a str,
    current: &'a str,
    selected: bool,
    width: usize,
) {
    let prefix = if selected {
        GLYPH_SELECTED
    } else {
        GLYPH_UNSELECTED
    };
    if selected {
        let bg = color_accent();
        let fg = color_bg();
        // Current value is inverted but lower-contrast, like an action description.
        let cur_fg = color_bg_panel();
        let mut spans = vec![
            Span::styled(
                prefix,
                Style::default().bg(bg).fg(fg).add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                format!("{:20} ", label),
                Style::default().bg(bg).fg(fg).add_modifier(Modifier::BOLD),
            ),
            Span::styled(current, Style::default().bg(bg).fg(cur_fg)),
        ];
        pad_selected_line(&mut spans, width, bg);
        lines.push(Line::from(spans));
    } else {
        let name_st = style_popup_unselected().add_modifier(Modifier::BOLD);
        let cur_st = Style::default().fg(color_dim());
        lines.push(Line::from(vec![
            Span::styled(prefix, style_popup_unselected()),
            Span::styled(format!("{:20} ", label), name_st),
            Span::styled(current, cur_st),
        ]));
    }
}

/// Pad a selected item's span list so its active background fills the whole line.
fn pad_selected_line(spans: &mut Vec<Span>, width: usize, bg: ratatui::style::Color) {
    let used: usize = spans.iter().map(|s| s.content.chars().count()).sum();
    let pad = width.saturating_sub(used);
    if pad > 0 {
        spans.push(Span::styled(" ".repeat(pad), Style::default().bg(bg)));
    }
}
