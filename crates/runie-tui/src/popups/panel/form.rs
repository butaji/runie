//! Form-style panel rendering (save/load/delete session, etc.)

use ratatui::{
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Paragraph, Wrap},
    Frame,
};
use unicode_width::UnicodeWidthChar;
use runie_core::dialog::{Panel, PanelItem};

use crate::theme::{color_accent, style_hint, style_placeholder, style_thinking};
use crate::ui::parse_hint_spans;

use super::{pad_to_width, setup_popup, style_border};

pub(super) fn render_form(f: &mut Frame, panel: &Panel, root_closable: bool) {
    let inner = setup_popup(f, &panel.title);
    let inner_h = inner.height as usize;
    let inner_w = inner.width as usize;

    let hint_lines = hint_lines(panel, root_closable);
    let button_line = build_button_line(panel, inner_w);
    let mut body = build_body(panel, inner_w);

    // Reserve 1 line for buttons + 2 lines for hints
    let body_h = inner_h.saturating_sub(3);
    if body.len() > body_h {
        body.truncate(body_h);
    }
    while body.len() < body_h {
        body.push(Line::from(""));
    }

    body.push(button_line);
    body.extend(hint_lines);
    let bg = Style::default().bg(crate::theme::color_bg_panel());
    f.render_widget(
        Paragraph::new(body).style(bg).wrap(Wrap { trim: false }),
        inner,
    );
}

fn hint_lines(panel: &Panel, root_closable: bool) -> Vec<Line<'_>> {
    let on_button = panel
        .selected_item()
        .is_some_and(|i| matches!(i, PanelItem::Action { .. }));
    let on_submit = panel
        .selected_item()
        .is_some_and(|i| matches!(i, PanelItem::FormSubmit));
    let close_hint = if root_closable { " · esc close" } else { "" };
    let hint_text = if on_button || on_submit {
        format!("↑↓ navigate · enter activate{}", close_hint)
    } else {
        format!("↑↓ navigate · enter edit{}", close_hint)
    };
    vec![Line::from(""), Line::from(parse_hint_spans(&hint_text))]
}

fn build_body(panel: &Panel, inner_w: usize) -> Vec<Line<'_>> {
    let mut body: Vec<Line> = Vec::new();
    push_header(&mut body, inner_w);
    let mut nav_idx = 0usize;
    let field_indices: Vec<usize> = field_indices(panel);
    let field_count = field_indices.len();
    for (raw_i, item) in panel.items.iter().enumerate() {
        push_body_item(
            &mut body,
            item,
            raw_i,
            panel.selected,
            inner_w,
            &field_indices,
            field_count,
            &mut nav_idx,
        );
    }
    body
}

// allow: orthogonal form layout params — bundled for panel item rendering
#[allow(clippy::too_many_arguments)]
fn push_body_item<'a>(
    body: &mut Vec<Line<'a>>,
    item: &'a PanelItem,
    raw_i: usize,
    selected: usize,
    inner_w: usize,
    field_indices: &[usize],
    field_count: usize,
    nav_idx: &mut usize,
) {
    match item {
        PanelItem::Header(text) => {
            body.push(Line::from(format!("  {}", text)).style(style_thinking()))
        }
        PanelItem::Separator => body.push(Line::from("")),
        PanelItem::FormField { .. } => {
            push_form_field_body(
                body,
                raw_i,
                selected,
                inner_w,
                field_indices,
                field_count,
                nav_idx,
                item,
            );
        }
        PanelItem::Toggle { .. } => {
            push_toggle_body(body, selected, nav_idx, item);
        }
        PanelItem::Action { .. } | PanelItem::Command { .. } | PanelItem::FormSubmit => {
            *nav_idx += 1
        }
        PanelItem::Select { .. } => {}
    }
}

// allow: orthogonal form layout params — bundled for field rendering context
#[allow(clippy::too_many_arguments)]
fn push_form_field_body<'a>(
    body: &mut Vec<Line<'a>>,
    raw_i: usize,
    selected: usize,
    inner_w: usize,
    field_indices: &[usize],
    field_count: usize,
    nav_idx: &mut usize,
    item: &'a PanelItem,
) {
    if let PanelItem::FormField {
        label,
        value,
        placeholder,
        cursor_pos,
        ..
    } = item
    {
        let field_pos = field_indices.iter().position(|&i| i == raw_i).unwrap_or(0);
        push_field(
            body,
            field_pos + 1,
            field_count,
            label,
            value,
            placeholder,
            *cursor_pos,
            *nav_idx == selected,
            inner_w,
        );
        body.push(Line::from(""));
        *nav_idx += 1;
    }
}

fn push_toggle_body<'a>(
    body: &mut Vec<Line<'a>>,
    selected: usize,
    nav_idx: &mut usize,
    item: &'a PanelItem,
) {
    if let PanelItem::Toggle { label, value, .. } = item {
        push_toggle_item(body, label, *value, *nav_idx == selected);
        *nav_idx += 1;
    }
}

/// Render a toggle (checkbox) line in the form body. Toggle items are
/// the universal checkbox in the DSL — no separate Checkbox variant.
fn push_toggle_item<'a>(body: &mut Vec<Line<'a>>, label: &'a str, checked: bool, is_active: bool) {
    let mark = if checked { "[x]" } else { "[ ]" };
    let text = format!("  {} {}", mark, label);
    let style = if is_active {
        Style::default()
            .fg(color_accent())
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default()
    };
    body.push(Line::from(text).style(style));
}

/// Build spans for a single button label with accelerator underline.
fn make_button_spans(label: &str, is_active: bool) -> Vec<Span<'_>> {
    use runie_core::dialog::{parse_accel, strip_accel};
    let stripped = strip_accel(label);
    let display = format!("  {}  ", stripped);
    let btn_style = if is_active {
        Style::default()
            .bg(color_accent())
            .fg(Color::Black)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().bg(Color::DarkGray).fg(Color::White)
    };
    let accel = parse_accel(label);
    let mut found_accel = false;
    let mut spans = Vec::new();
    for ch in display.chars() {
        if accel == Some(ch) && !found_accel {
            spans.push(Span::styled(
                ch.to_string(),
                btn_style.add_modifier(Modifier::UNDERLINED),
            ));
            found_accel = true;
        } else {
            spans.push(Span::styled(ch.to_string(), btn_style));
        }
    }
    spans.push(Span::styled("  ", Style::default()));
    spans
}

/// Build a single right-aligned line containing all form buttons.
fn build_button_line(panel: &Panel, inner_w: usize) -> Line<'_> {
    let mut button_spans: Vec<Span> = Vec::new();
    let mut nav_idx = 0usize;

    for item in panel.items.iter() {
        let (label, is_active) = match item {
            PanelItem::Action { label, .. } => {
                let active = nav_idx == panel.selected;
                nav_idx += 1;
                (Some(label.as_str()), active)
            }
            PanelItem::FormSubmit => {
                let active = nav_idx == panel.selected;
                nav_idx += 1;
                (Some("Submit"), active)
            }
            _ if item.is_navigable() => {
                nav_idx += 1;
                continue;
            }
            _ => continue,
        };
        let Some(label) = label else { continue };
        button_spans.extend(make_button_spans(label, is_active));
    }

    // Trim trailing gap and right-align
    while button_spans.last().is_some_and(|s| s.content == "  ") {
        button_spans.pop();
    }
    let total_chars: usize = button_spans.iter().map(|s| s.content.chars().count()).sum();
    let pad = inner_w.saturating_sub(total_chars);
    let mut spans = vec![Span::styled(" ".repeat(pad), Style::default())];
    spans.extend(button_spans);
    Line::from(spans)
}

fn push_header(lines: &mut Vec<Line>, inner_w: usize) {
    lines.push(Line::from("  Fill in the form and press Enter to submit").style(style_hint()));
    lines.push(Line::from("─".repeat(inner_w)).style(style_hint()));
    lines.push(Line::from(""));
}

fn field_indices(panel: &Panel) -> Vec<usize> {
    panel
        .items
        .iter()
        .enumerate()
        .filter(|(_, i)| matches!(i, PanelItem::FormField { .. }))
        .map(|(i, _)| i)
        .collect()
}

// allow: orthogonal field rendering params — bundled for complete field rendering
#[allow(clippy::too_many_arguments)]
fn push_field<'a>(
    lines: &mut Vec<Line<'a>>,
    field_num: usize,
    total: usize,
    label: &'a str,
    value: &str,
    placeholder: &str,
    cursor_pos: usize,
    is_active: bool,
    inner_w: usize,
) {
    lines.push(field_label_line(field_num, total, label, is_active));

    let (top, mid_spans, bot) =
        build_input_box(value, placeholder, cursor_pos, is_active, inner_w);
    lines.push(Line::from(top).style(style_border()));
    lines.push(Line::from(mid_spans));
    lines.push(Line::from(bot).style(style_border()));
}

fn field_label_line<'a>(
    field_num: usize,
    total: usize,
    label: &'a str,
    is_active: bool,
) -> Line<'a> {
    let text = if total > 1 {
        format!("  {}. {} ({}/{})", field_num, label, field_num, total)
    } else {
        format!("  {}. {}", field_num, label)
    };
    let style = if is_active {
        Style::default()
            .fg(color_accent())
            .add_modifier(Modifier::BOLD)
    } else {
        style_hint()
    };
    Line::from(text).style(style)
}

fn build_input_box<'a>(
    value: &str,
    placeholder: &str,
    cursor_pos: usize,
    is_active: bool,
    inner_w: usize,
) -> (String, Vec<Span<'a>>, String) {
    let box_w = inner_w.saturating_sub(6).max(12);
    let inner_avail = box_w.saturating_sub(2);
    let spans = input_display_spans(value, placeholder, cursor_pos, inner_avail, is_active);
    let top = format!("  ┌{}┐", "─".repeat(box_w - 2));
    let bot = format!("  └{}┘", "─".repeat(box_w - 2));
    let mut all_spans = vec![Span::styled("  │", style_border())];
    all_spans.extend(spans);
    all_spans.push(Span::styled("│", style_border()));
    (top, all_spans, bot)
}

fn input_display_spans(
    value: &str,
    placeholder: &str,
    cursor_pos: usize,
    avail: usize,
    is_active: bool,
) -> Vec<Span<'static>> {
    let val_style = input_value_style(value, is_active);
    let cursor_style = if value.is_empty() {
        style_placeholder()
    } else {
        val_style
    };

    if value.is_empty() {
        let text = if is_active {
            format!("{}▏", placeholder)
        } else {
            placeholder.to_string()
        };
        return vec![Span::styled(pad_to_width(&text, avail), cursor_style)];
    }

    let cursor_pos = cursor_pos.min(value.len());
    let before = &value[..cursor_pos];
    let after = &value[cursor_pos..];
    let before_w = runie_core::display_width::width(before) as usize;
    let after_w = runie_core::display_width::width(after) as usize;
    let scroll = compute_field_scroll(before_w, after_w, avail);

    build_field_spans(before, after, before_w, scroll, avail, val_style, cursor_style)
}

fn compute_field_scroll(before_w: usize, after_w: usize, avail: usize) -> usize {
    let total_w = before_w + 1 + after_w;
    if total_w <= avail {
        return 0;
    }
    let need = before_w + 1;
    if need <= avail {
        0
    } else {
        need - avail
    }
}

fn build_field_spans(
    before: &str,
    after: &str,
    before_w: usize,
    scroll: usize,
    avail: usize,
    val_style: Style,
    cursor_style: Style,
) -> Vec<Span<'static>> {
    let mut spans: Vec<Span<'static>> = Vec::new();
    let mut used = 0usize;

    if scroll > 0 {
        spans.push(Span::styled("…", val_style));
        used += 1;
    }

    let skip = scroll;
    let (prefix, prefix_used) = visible_segment(before, skip, avail.saturating_sub(used));
    spans.push(Span::styled(prefix, val_style));
    used += prefix_used;

    if used < avail && before_w + 1 > scroll {
        spans.push(Span::styled("▏", cursor_style));
        used += 1;
    }

    let remaining = avail.saturating_sub(used);
    let (suffix, suffix_used) = visible_segment(after, 0, remaining);
    spans.push(Span::styled(suffix, val_style));
    used += suffix_used;

    if used < avail {
        spans.push(Span::styled(" ".repeat(avail - used), val_style));
    }

    spans
}

fn visible_segment(s: &str, skip_w: usize, max_w: usize) -> (String, usize) {
    let mut result = String::new();
    let mut skipped = 0usize;
    let mut used = 0usize;
    for ch in s.chars() {
        let w = ch.width().unwrap_or(0);
        if skipped + w <= skip_w {
            skipped += w;
            continue;
        }
        if used + w > max_w {
            break;
        }
        result.push(ch);
        used += w;
    }
    (result, used)
}

fn input_value_style(value: &str, is_active: bool) -> Style {
    if value.is_empty() {
        style_placeholder()
    } else if is_active {
        Style::default()
            .fg(Color::White)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default()
    }
}
