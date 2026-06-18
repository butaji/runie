//! List-style panel rendering (command palette, settings, model selector, etc.)

use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
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

use super::{compute_scrolling, hotkey_area, hotkey_area_height, setup_popup, ScrollLayout};

pub(super) fn render_list(
    f: &mut Frame,
    panel: &Panel,
    show_breadcrumb: bool,
    root_closable: bool,
) {
    let inner = setup_popup(f, &panel.title);
    let inner_w = inner.width as usize;
    let (header_lines, header_height) = build_header(panel, inner_w);
    let items_area = Rect {
        x: inner.x,
        y: inner.y + header_height,
        width: inner.width,
        height: inner
            .height
            .saturating_sub(header_height + hotkey_area_height()),
    };
    let item_width = item_width(panel, &items_area, inner_w);
    let (item_lines, selected_line) = build_items(panel, item_width);
    let scroll = compute_scrolling(&items_area, item_lines.len(), selected_line);
    let bg = Style::default().bg(color_bg_panel());

    render_header(f, header_lines, header_height, inner, bg);
    render_item_list(f, item_lines, scroll, bg);
    render_hotkeys(f, show_breadcrumb, root_closable, inner, bg);
}

fn item_width(panel: &Panel, items_area: &Rect, inner_w: usize) -> usize {
    let total = panel.filtered_items().len();
    let visible = items_area.height as usize;
    if total > visible {
        inner_w.saturating_sub(1)
    } else {
        inner_w
    }
}

fn render_header(
    f: &mut Frame,
    header_lines: Vec<Line<'_>>,
    header_height: u16,
    inner: Rect,
    bg: Style,
) {
    let header_area = Rect {
        height: header_height,
        ..inner
    };
    f.render_widget(Paragraph::new(header_lines).style(bg), header_area);
}

fn render_item_list(f: &mut Frame, item_lines: Vec<Line<'_>>, scroll: ScrollLayout, bg: Style) {
    f.render_widget(
        Paragraph::new(item_lines)
            .style(bg)
            .wrap(Wrap { trim: false })
            .scroll((scroll.offset as u16, 0)),
        scroll.area,
    );
    if scroll.show_bar {
        render_scrollbar(
            f,
            scroll.bar_area,
            scroll.total,
            scroll.offset as u16,
            scroll.area.height as usize,
        );
    }
}

fn render_hotkeys(
    f: &mut Frame,
    show_breadcrumb: bool,
    root_closable: bool,
    inner: Rect,
    bg: Style,
) {
    let hotkey_text = match (show_breadcrumb, root_closable) {
        (true, true) => "↑↓ navigate · enter select · ← back · esc close",
        (true, false) => "↑↓ navigate · enter select · ← back",
        (false, true) => "↑↓ navigate · enter select · esc close",
        (false, false) => "↑↓ navigate · enter select",
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
        PanelItem::Command { name, desc, .. } => push_command(lines, name, desc, selected, width),
        PanelItem::Toggle { label, value, .. } => {
            push_toggle(lines, label, *value, selected, width)
        }
        PanelItem::Select { label, current, .. } => {
            push_select(lines, label, current, selected, width)
        }
        _ => {}
    }
}

/// Split a label like "readonly Toggle read-only mode" into ("readonly", "Toggle read-only mode").
fn split_label(label: &str) -> (&str, &str) {
    match label.find(' ') {
        Some(i) => (&label[..i], &label[i + 1..]),
        None => (label, ""),
    }
}

fn push_command<'a>(
    lines: &mut Vec<Line<'a>>,
    name: &'a str,
    desc: &'a str,
    selected: bool,
    width: usize,
) {
    push_named_item(lines, name, desc, selected, width);
}

fn push_action<'a>(lines: &mut Vec<Line<'a>>, label: &'a str, selected: bool, width: usize) {
    let (name, desc) = split_label(label);
    push_named_item(lines, name, desc, selected, width);
}

fn push_named_item<'a>(
    lines: &mut Vec<Line<'a>>,
    name: &'a str,
    desc: &'a str,
    selected: bool,
    width: usize,
) {
    let prefix = selection_glyph(selected);
    if selected {
        let bg = color_accent();
        let fg = color_bg();
        let desc_fg = color_bg_panel();
        let base = selected_style(bg, fg).add_modifier(Modifier::BOLD);
        let mut spans = vec![Span::styled(prefix, base), Span::styled(name, base)];
        if !desc.is_empty() {
            spans.push(Span::styled(
                format!(" {}", desc),
                Style::default().bg(bg).fg(desc_fg),
            ));
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
            spans.push(Span::styled(format!(" {}", desc), desc_st));
        }
        lines.push(Line::from(spans));
    }
}

fn push_toggle(lines: &mut Vec<Line>, label: &str, value: bool, selected: bool, width: usize) {
    let prefix = selection_glyph(selected);
    let mark = if value { "[x]" } else { "[ ]" };
    let text = format!("{} {}", mark, label);
    if selected {
        let bg = color_accent();
        let fg = color_bg();
        let st = selected_style(bg, fg).add_modifier(Modifier::BOLD);
        let mut spans = vec![Span::styled(prefix, st), Span::styled(text, st)];
        pad_selected_line(&mut spans, width, bg);
        lines.push(Line::from(spans));
    } else {
        lines.push(Line::from(format!("{}{}", prefix, text)).style(style_popup_unselected()));
    }
}

fn push_select<'a>(
    lines: &mut Vec<Line<'a>>,
    label: &'a str,
    current: &'a str,
    selected: bool,
    width: usize,
) {
    let prefix = selection_glyph(selected);
    if selected {
        let bg = color_accent();
        let fg = color_bg();
        let cur_fg = color_bg_panel();
        let base = selected_style(bg, fg).add_modifier(Modifier::BOLD);
        let mut spans = vec![
            Span::styled(prefix, base),
            Span::styled(format!("{:20} ", label), base),
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
fn pad_selected_line(spans: &mut Vec<Span>, width: usize, bg: Color) {
    let used: usize = spans.iter().map(|s| s.content.chars().count()).sum();
    let pad = width.saturating_sub(used);
    if pad > 0 {
        spans.push(Span::styled(" ".repeat(pad), Style::default().bg(bg)));
    }
}

fn selection_glyph(selected: bool) -> &'static str {
    if selected {
        GLYPH_SELECTED
    } else {
        GLYPH_UNSELECTED
    }
}

fn selected_style(bg: Color, fg: Color) -> Style {
    Style::default().bg(bg).fg(fg)
}
