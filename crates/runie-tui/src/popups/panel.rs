use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Paragraph, Wrap},
    Frame,
};
use runie_core::Snapshot;
use runie_core::dialog::{PanelItem, Panel};

use crate::theme::{
    GLYPH_SELECTED, GLYPH_UNSELECTED, block_popup, style_popup_selected,
    style_popup_unselected, style_hint, style_thinking, style_user, style_placeholder,
    color_bg_panel, color_accent, color_border,
};
use crate::ui::{parse_hint_spans, render_scrollbar};
use crate::popups::{palette_popup_rect, clear_panel_bg};

// ============================================================================
// Entry point
// ============================================================================

pub fn panel_dialog(f: &mut Frame, snap: &Snapshot) {
    let stack = match &snap.dialog {
        Some(runie_core::commands::DialogState::PanelStack(stack)) => stack,
        _ => return,
    };
    let panel = match stack.current() {
        Some(p) => p,
        None => return,
    };
    if panel.is_form() {
        render_form(f, panel);
        return;
    }
    render_list(f, panel, stack.len() > 1);
}

// ============================================================================
// List-style rendering (commands, settings, scoped models, etc.)
// ============================================================================

fn render_list(f: &mut Frame, panel: &Panel, show_breadcrumb: bool) {
    let inner = setup_popup(f, &panel.title);
    let (header_lines, header_height) = build_list_header(panel, inner.width as usize);
    let header_area = Rect { height: header_height, ..inner };
    let items_area = below(&inner, header_height);
    let (item_lines, selected_line) = build_list_items(panel);
    let total = item_lines.len();
    let (scroll_area, scrollbar_area, show_scrollbar, scroll_offset) =
        compute_scrolling(&items_area, total, selected_line);
    let bg = Style::default().bg(color_bg_panel());
    f.render_widget(Paragraph::new(header_lines).style(bg), header_area);
    f.render_widget(
        Paragraph::new(item_lines).style(bg).wrap(Wrap { trim: false })
            .scroll((scroll_offset as u16, 0)),
        scroll_area,
    );
    if show_scrollbar {
        render_scrollbar(f, scrollbar_area, total, scroll_offset as u16, scroll_area.height as usize);
    }
    let hotkey_text = if show_breadcrumb {
        "↑↓ navigate · enter select · ← back · esc close"
    } else { "↑↓ navigate · enter select · esc close" };
    f.render_widget(
        Paragraph::new(vec![Line::from(""), Line::from(parse_hint_spans(hotkey_text))]).style(bg),
        hotkey_area(&inner),
    );
}

fn build_list_header(panel: &Panel, inner_w: usize) -> (Vec<Line>, u16) {
    let mut lines: Vec<Line> = Vec::new();
    if panel.filterable {
        lines.push(Line::from(format!("❯ {}", panel.filter)).style(style_user()));
    }
    lines.push(Line::from("─".repeat(inner_w)).style(style_hint()));
    let h = lines.len() as u16;
    (lines, h)
}

fn build_list_items(panel: &Panel) -> (Vec<Line>, Option<usize>) {
    let filtered = panel.filtered_items();
    let mut item_lines: Vec<Line> = Vec::new();
    let mut selected_line: Option<usize> = None;
    let mut nav_idx = 0;

    for item in filtered.iter() {
        match item {
            PanelItem::Header(text) => {
                item_lines.push(Line::from(format!("  {}", text)).style(style_thinking()));
            }
            PanelItem::Separator => { item_lines.push(Line::from("")); }
            PanelItem::Action { label, .. } => {
                if nav_idx == panel.selected { selected_line = Some(item_lines.len()); }
                push_action(&mut item_lines, label, nav_idx == panel.selected);
                nav_idx += 1;
            }
            PanelItem::Toggle { label, value, .. } => {
                if nav_idx == panel.selected { selected_line = Some(item_lines.len()); }
                push_toggle(&mut item_lines, label, *value, nav_idx == panel.selected);
                nav_idx += 1;
            }
            PanelItem::Select { label, current, .. } => {
                if nav_idx == panel.selected { selected_line = Some(item_lines.len()); }
                push_select(&mut item_lines, label, current, nav_idx == panel.selected);
                nav_idx += 1;
            }
            PanelItem::FormField { .. } | PanelItem::FormSubmit => {
                // Forms use render_form; skip here.
            }
        }
    }
    (item_lines, selected_line)
}

fn push_action(lines: &mut Vec<Line>, label: &str, selected: bool) {
    let prefix = if selected { GLYPH_SELECTED } else { GLYPH_UNSELECTED };
    let style = if selected { style_popup_selected() } else { style_popup_unselected() };
    lines.push(Line::from(format!("{}{}", prefix, label)).style(style));
}

fn push_toggle(lines: &mut Vec<Line>, label: &str, value: bool, selected: bool) {
    let prefix = if selected { GLYPH_SELECTED } else { GLYPH_UNSELECTED };
    let style = if selected { style_popup_selected() } else { style_popup_unselected() };
    let mark = if value { "[x]" } else { "[ ]" };
    lines.push(Line::from(format!("{}{} {}", prefix, mark, label)).style(style));
}

fn push_select(lines: &mut Vec<Line>, label: &str, current: &str, selected: bool) {
    let prefix = if selected { GLYPH_SELECTED } else { GLYPH_UNSELECTED };
    let style = if selected { style_popup_selected() } else { style_popup_unselected() };
    lines.push(Line::from(format!("{}{:20} {}", prefix, label, current)).style(style));
}

// ============================================================================
// Form rendering — distinct layout
// ============================================================================

fn render_form(f: &mut Frame, panel: &Panel) {
    let inner = setup_popup(f, &panel.title);
    let inner_h = inner.height as usize;
    let inner_w = inner.width as usize;

    let on_submit = panel.selected_item().map_or(false, |i| matches!(i, PanelItem::FormSubmit));
    let hint_text = if on_submit { "↑↓ navigate · enter submit · esc close" }
                    else { "↑↓ navigate · enter edit · esc close" };
    let hint_lines: Vec<Line> = vec![Line::from(""), Line::from(parse_hint_spans(hint_text))];

    // Build body
    let mut body: Vec<Line> = Vec::new();
    let active_field = active_field_index(panel);
    push_form_header(&mut body, inner_w);
    let fields: Vec<usize> = form_field_indices(panel);
    let total = fields.len();
    for (i, raw_i) in fields.iter().enumerate() {
        if let PanelItem::FormField { label, value, placeholder, .. } = &panel.items[*raw_i] {
            let is_active = i == active_field;
            push_form_field(&mut body, i + 1, total, label, value, placeholder, is_active, inner_w);
            body.push(Line::from(""));
        }
    }
    push_form_submit(&mut body, on_submit, inner_w);
    body.push(Line::from(""));

    // Reserve 2 lines for hotkey hint at the very bottom
    let body_h = inner_h.saturating_sub(2);
    if body.len() > body_h { body.truncate(body_h); }
    while body.len() < body_h { body.push(Line::from("")); }

    let mut lines = body;
    lines.extend(hint_lines);
    let bg = Style::default().bg(color_bg_panel());
    f.render_widget(
        Paragraph::new(lines).style(bg).wrap(Wrap { trim: false }),
        inner,
    );
}

fn push_form_header(lines: &mut Vec<Line>, inner_w: usize) {
    lines.push(Line::from("  Fill in the form and press Enter to submit").style(style_hint()));
    lines.push(Line::from("─".repeat(inner_w)).style(style_hint()));
    lines.push(Line::from(""));
}

fn form_field_indices(panel: &Panel) -> Vec<usize> {
    panel.items.iter().enumerate()
        .filter(|(_, i)| matches!(i, PanelItem::FormField { .. }))
        .map(|(i, _)| i)
        .collect()
}

/// Returns the index of the currently active form field (0-based), or 0 if
/// no field is active.
fn active_field_index(panel: &Panel) -> usize {
    let field_indices: Vec<usize> = panel.items.iter().enumerate()
        .filter(|(_, i)| matches!(i, PanelItem::FormField { .. }))
        .map(|(i, _)| i)
        .collect();
    if field_indices.is_empty() { return 0; }
    let mut nav_count = 0;
    for (raw_i, item) in panel.items.iter().enumerate() {
        if matches!(item, PanelItem::FormField { .. }) {
            if nav_count == panel.selected {
                return field_indices.iter().position(|&i| i == raw_i).unwrap_or(0);
            }
            nav_count += 1;
        } else if item.is_navigable() {
            nav_count += 1;
        }
    }
    0
}

fn push_form_field(
    lines: &mut Vec<Line>,
    field_num: usize, total: usize,
    label: &str, value: &str, placeholder: &str,
    is_active: bool, inner_w: usize,
) {
    // Label line
    let label_text = if total > 1 {
        format!("  {} {} ({}/{})", circled_number(field_num), label, field_num, total)
    } else {
        format!("  {} {}", circled_number(field_num), label)
    };
    let label_style = if is_active {
        Style::default().fg(color_accent()).add_modifier(Modifier::BOLD)
    } else {
        style_hint()
    };
    lines.push(Line::from(label_text).style(label_style));

    // Input box (3 lines: top border, content, bottom border)
    let (top, mid_spans, bot) = build_input_box(value, placeholder, is_active, inner_w);
    lines.push(Line::from(top).style(style_border()));
    lines.push(Line::from(mid_spans));
    lines.push(Line::from(bot).style(style_border()));
}

fn build_input_box<'a>(
    value: &str, placeholder: &str,
    is_active: bool, inner_w: usize,
) -> (String, Vec<Span<'a>>, String) {
    let box_w = inner_w.saturating_sub(6).max(12);
    let inner_avail = box_w.saturating_sub(2);
    let display = if value.is_empty() { placeholder.to_string() } else { value.to_string() };
    let (shown, overflow) = truncate(&display, inner_avail.saturating_sub(1));
    let inner_text = if overflow { format!("{}…", shown) } else { shown };
    let inner_text = if is_active { format!("{}▏", inner_text) } else { inner_text };
    let inner_padded = pad_to_width(&inner_text, inner_avail);
    let val_style = if value.is_empty() {
        style_placeholder()
    } else if is_active {
        Style::default().fg(Color::White).add_modifier(Modifier::BOLD)
    } else {
        Style::default()
    };
    let top = format!("  ┌{}┐", "─".repeat(box_w - 2));
    let bot = format!("  └{}┘", "─".repeat(box_w - 2));
    let spans = vec![
        Span::styled("  │", style_border()),
        Span::styled(inner_padded, val_style),
        Span::styled("│", style_border()),
    ];
    (top, spans, bot)
}

fn push_form_submit(lines: &mut Vec<Line>, is_active: bool, inner_w: usize) {
    let label = " Submit ";
    let box_w = label.chars().count() + 4;
    let top = format!("┌{}┐", "─".repeat(box_w - 2));
    let middle = format!("│{}│", label);
    let bottom = format!("└{}┘", "─".repeat(box_w - 2));
    let border_style = if is_active {
        Style::default().fg(color_accent()).add_modifier(Modifier::BOLD)
    } else {
        style_border()
    };
    let middle_style = if is_active {
        Style::default().bg(color_accent()).fg(Color::Black).add_modifier(Modifier::BOLD)
    } else {
        Style::default()
    };
    let pad_total = inner_w.saturating_sub(box_w + 2);
    let pad = " ".repeat(pad_total / 2);
    lines.push(Line::from(vec![Span::styled(pad.clone(), Style::default()), Span::styled(top, border_style)]));
    lines.push(Line::from(vec![Span::styled(pad.clone(), Style::default()), Span::styled(middle, middle_style)]));
    lines.push(Line::from(vec![Span::styled(pad, Style::default()), Span::styled(bottom, border_style)]));
}

// ============================================================================
// Shared layout helpers
// ============================================================================

fn setup_popup(f: &mut Frame, title: &str) -> Rect {
    let popup_area = palette_popup_rect(f.area());
    clear_panel_bg(f, popup_area);
    let title_owned = format!(" {} ", title);
    let block = block_popup(&title_owned);
    let inner = block.inner(popup_area);
    f.render_widget(Paragraph::new("").block(block), popup_area);
    f.buffer_mut().set_style(inner, Style::default().bg(color_bg_panel()));
    inner
}

fn below(area: &Rect, header_h: u16) -> Rect {
    Rect { x: area.x, y: area.y + header_h, height: area.height.saturating_sub(header_h), width: area.width }
}

fn hotkey_area(inner: &Rect) -> Rect {
    Rect { x: inner.x, y: inner.y + inner.height.saturating_sub(2), width: inner.width, height: 2 }
}

fn compute_scrolling(area: &Rect, total: usize, selected: Option<usize>)
    -> (Rect, Rect, bool, usize)
{
    let visible = area.height as usize;
    let show_scrollbar = total > visible;
    let items_width = if show_scrollbar { area.width.saturating_sub(1) } else { area.width };
    let scroll_area = Rect { width: items_width, ..*area };
    let scrollbar_area = Rect {
        x: area.x + items_width, y: area.y, width: 1, height: area.height,
    };
    let scroll_offset = if let Some(sel) = selected {
        if total <= visible { 0 } else {
            sel.saturating_sub(visible / 2).min(total.saturating_sub(visible))
        }
    } else { 0 };
    (scroll_area, scrollbar_area, show_scrollbar, scroll_offset)
}

fn style_border() -> Style {
    Style::default().fg(color_border())
}

fn circled_number(n: usize) -> String {
    match n {
        1 => "①".into(), 2 => "②".into(), 3 => "③".into(), 4 => "④".into(),
        5 => "⑤".into(), 6 => "⑥".into(), 7 => "⑦".into(), 8 => "⑧".into(),
        9 => "⑨".into(), _ => format!("{}.", n),
    }
}

fn truncate(s: &str, max: usize) -> (String, bool) {
    if s.chars().count() <= max {
        (s.to_string(), false)
    } else {
        let truncated: String = s.chars().take(max.saturating_sub(1)).collect();
        (truncated, true)
    }
}

fn pad_to_width(s: &str, width: usize) -> String {
    let char_count = s.chars().count();
    if char_count >= width {
        s.to_string()
    } else {
        format!("{}{}", s, " ".repeat(width - char_count))
    }
}

