//! Form-style panel rendering (save/load/delete session, etc.)

use ratatui::{
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Paragraph, Wrap},
    Frame,
};
use runie_core::dialog::{Panel, PanelItem};

use crate::theme::{color_accent, style_hint, style_placeholder, style_thinking};
use crate::ui::parse_hint_spans;

use super::{circled_number, pad_to_width, setup_popup, style_border, truncate};

pub(super) fn render_form(f: &mut Frame, panel: &Panel) {
    let inner = setup_popup(f, &panel.title);
    let inner_h = inner.height as usize;
    let inner_w = inner.width as usize;

    let hint_lines = hint_lines(panel);
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

fn hint_lines(panel: &Panel) -> Vec<Line<'_>> {
    let on_button = panel
        .selected_item()
        .is_some_and(|i| matches!(i, PanelItem::Action { .. }));
    let on_submit = panel
        .selected_item()
        .is_some_and(|i| matches!(i, PanelItem::FormSubmit));
    let hint_text = if on_button || on_submit {
        "↑↓ navigate · enter activate · esc close"
    } else {
        "↑↓ navigate · enter edit · esc close"
    };
    vec![Line::from(""), Line::from(parse_hint_spans(hint_text))]
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
        PanelItem::FormField {
            label,
            value,
            placeholder,
            ..
        } => push_form_field_item(
            body,
            raw_i,
            selected,
            inner_w,
            field_indices,
            field_count,
            nav_idx,
            label,
            value,
            placeholder,
        ),
        PanelItem::Toggle { label, value, .. } => {
            push_toggle_item(body, label, *value, *nav_idx == selected);
            *nav_idx += 1;
        }
        PanelItem::Action { .. } | PanelItem::Command { .. } | PanelItem::FormSubmit => {
            *nav_idx += 1
        }
        PanelItem::Select { .. } => {}
    }
}

/// Render a toggle (checkbox) line in the form body. Toggle items are
/// the universal checkbox in the DSL — no separate Checkbox variant.
fn push_toggle_item<'a>(body: &mut Vec<Line<'a>>, label: &'a str, checked: bool, is_active: bool) {
    let mark = if checked { "[✓]" } else { "[ ]" };
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

#[allow(clippy::too_many_arguments)]
fn push_form_field_item<'a>(
    body: &mut Vec<Line<'a>>,
    raw_i: usize,
    selected: usize,
    inner_w: usize,
    field_indices: &[usize],
    field_count: usize,
    nav_idx: &mut usize,
    label: &'a str,
    value: &str,
    placeholder: &str,
) {
    let field_pos = field_indices.iter().position(|&i| i == raw_i).unwrap_or(0);
    push_field(
        body,
        field_pos + 1,
        field_count,
        label,
        value,
        placeholder,
        *nav_idx == selected,
        inner_w,
    );
    body.push(Line::from(""));
    *nav_idx += 1;
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

#[allow(clippy::too_many_arguments)]
fn push_field<'a>(
    lines: &mut Vec<Line<'a>>,
    field_num: usize,
    total: usize,
    label: &'a str,
    value: &str,
    placeholder: &str,
    is_active: bool,
    inner_w: usize,
) {
    lines.push(field_label_line(field_num, total, label, is_active));

    let (top, mid_spans, bot) = build_input_box(value, placeholder, is_active, inner_w);
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
        format!(
            "  {} {} ({}/{})",
            circled_number(field_num),
            label,
            field_num,
            total
        )
    } else {
        format!("  {} {}", circled_number(field_num), label)
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
    is_active: bool,
    inner_w: usize,
) -> (String, Vec<Span<'a>>, String) {
    let box_w = inner_w.saturating_sub(6).max(12);
    let inner_avail = box_w.saturating_sub(2);
    let inner_text = input_display_text(value, placeholder, inner_avail, is_active);
    let val_style = input_value_style(value, is_active);
    let top = format!("  ┌{}┐", "─".repeat(box_w - 2));
    let bot = format!("  └{}┘", "─".repeat(box_w - 2));
    let spans = vec![
        Span::styled("  │", style_border()),
        Span::styled(inner_text, val_style),
        Span::styled("│", style_border()),
    ];
    (top, spans, bot)
}

fn input_display_text(value: &str, placeholder: &str, avail: usize, is_active: bool) -> String {
    let display = if value.is_empty() {
        placeholder.to_string()
    } else {
        value.to_string()
    };
    let (shown, overflow) = truncate(&display, avail.saturating_sub(1));
    let text = if overflow {
        format!("{}…", shown)
    } else {
        shown
    };
    let text = if is_active {
        format!("{}▏", text)
    } else {
        text
    };
    pad_to_width(&text, avail)
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
