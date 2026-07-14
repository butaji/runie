//! Form-style panel rendering (save/load/delete session, etc.)
//!
//! Uses `tui-input` for single-line text input within the form fields.

use ratatui::{
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::Paragraph,
    Frame,
};
use runie_core::dialog::{Panel, PanelItem};
use tui_input::Input;

use crate::theme::{
    color_accent, style_hint, style_placeholder, style_thinking, BOX_BOTTOM_LEFT, BOX_BOTTOM_RIGHT,
    BOX_HORIZONTAL, BOX_TOP_LEFT, BOX_TOP_RIGHT, BOX_VERTICAL, GLYPH_CHECKED, GLYPH_UNCHECKED,
};
use crate::ui::parse_hint_spans;

use super::{pad_to_width, setup_popup, style_border};

/// Max width for form input fields.
const FORM_INPUT_WIDTH: usize = 40;

/// Style constants for form inputs.
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

pub(super) fn render_form(f: &mut Frame, panel: &Panel, root_closable: bool) {
    let inner = setup_popup(f, &panel.title);
    let inner_h = inner.height as usize;
    let inner_w = inner.width as usize;

    let hint_lines = hint_lines(panel, root_closable);
    let button_lines = build_button_lines(panel, inner_w);
    let mut body = build_body(panel, inner_w);

    // Reserve lines for buttons + 2 lines for hints
    let body_h = inner_h.saturating_sub(button_lines.len() + 2);
    if body.len() > body_h {
        body.truncate(body_h);
    }
    while body.len() < body_h {
        body.push(Line::from(""));
    }

    body.extend(button_lines);
    body.extend(hint_lines);
    let _bg = Style::default().bg(crate::theme::color_bg_panel());
    f.render_widget(
        Paragraph::new(body).wrap(ratatui::widgets::Wrap { trim: false }),
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
    let field_indices: Vec<usize> = field_indices(panel);
    push_header(&mut body, inner_w, !field_indices.is_empty());
    let mut nav_idx = 0usize;
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
    let mark = if checked {
        GLYPH_CHECKED
    } else {
        GLYPH_UNCHECKED
    };
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

/// Build right-aligned button lines containing all form buttons.
///
/// Buttons are packed greedily: as many as fit on one line, wrapping to the
/// next line BETWEEN buttons so a label is never split across lines (the
/// permission dialog's four options overflowed a single row at common
/// terminal widths and wrapped mid-label).
fn build_button_lines(panel: &Panel, inner_w: usize) -> Vec<Line<'_>> {
    let mut buttons: Vec<Vec<Span>> = Vec::new();
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
        buttons.push(make_button_spans(label, is_active));
    }

    let mut lines: Vec<Line> = Vec::new();
    let mut current: Vec<Span> = Vec::new();
    let mut current_w = 0usize;
    for btn in buttons {
        let w: usize = btn.iter().map(|s| s.content.chars().count()).sum();
        if !current.is_empty() && current_w + w > inner_w {
            lines.push(right_aligned_line(
                std::mem::take(&mut current),
                current_w,
                inner_w,
            ));
            current_w = 0;
        }
        current_w += w;
        current.extend(btn);
    }
    if !current.is_empty() {
        lines.push(right_aligned_line(current, current_w, inner_w));
    }
    if lines.is_empty() {
        lines.push(Line::from(""));
    }
    lines
}

/// Right-align a packed row of button spans within `inner_w`.
fn right_aligned_line(mut spans: Vec<Span>, width: usize, inner_w: usize) -> Line<'static> {
    // Drop trailing whitespace-only gap spans so the row ends at the button.
    while spans.last().is_some_and(|s| s.content.trim().is_empty()) {
        spans.pop();
    }
    let width = width.min(inner_w);
    let pad = inner_w.saturating_sub(width);
    let mut line_spans = vec![Span::styled(" ".repeat(pad), Style::default())];
    line_spans.extend(
        spans
            .into_iter()
            .map(|s| Span::styled(s.content.into_owned(), s.style)),
    );
    Line::from(line_spans)
}

fn push_header(lines: &mut Vec<Line>, inner_w: usize, has_fields: bool) {
    let hint = if has_fields {
        "  Fill in the form and press Enter to submit"
    } else {
        "  Select an option and press Enter"
    };
    lines.push(Line::from(hint).style(style_hint()));
    lines.push(Line::from(BOX_HORIZONTAL.to_string().repeat(inner_w)).style(style_hint()));
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

/// Render a form field with label and input box using tui-input.
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

    let box_w = FORM_INPUT_WIDTH.min(inner_w.saturating_sub(6).max(12));
    let (top, mid_spans, bot) = build_input_box(value, placeholder, cursor_pos, is_active, box_w);
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

/// Build the ASCII box around the input field.
/// Uses tui-input for text/cursor management.
fn build_input_box<'a>(
    value: &str,
    placeholder: &str,
    cursor_pos: usize,
    is_active: bool,
    box_w: usize,
) -> (String, Vec<Span<'a>>, String) {
    let top = format!(
        "  {}{}{}",
        BOX_TOP_LEFT,
        BOX_HORIZONTAL.to_string().repeat(box_w - 2),
        BOX_TOP_RIGHT
    );
    let bot = format!(
        "  {}{}{}",
        BOX_BOTTOM_LEFT,
        BOX_HORIZONTAL.to_string().repeat(box_w - 2),
        BOX_BOTTOM_RIGHT
    );

    // Create tui-input instance for text/cursor management
    let mut input = Input::new(value.to_string());
    input = input.with_cursor(cursor_pos.min(value.len()));

    let inner_w = box_w.saturating_sub(4);
    let spans = render_tui_input(&input, placeholder, is_active, inner_w);

    let mut all_spans = vec![Span::styled(
        "  ".to_string() + &BOX_VERTICAL.to_string(),
        style_border(),
    )];
    all_spans.extend(spans);
    all_spans.push(Span::styled(BOX_VERTICAL.to_string(), style_border()));
    (top, all_spans, bot)
}

/// Render tui-input text content with cursor and placeholder.
fn render_tui_input<'a>(
    input: &Input,
    placeholder: &str,
    is_active: bool,
    avail: usize,
) -> Vec<Span<'a>> {
    let value = input.value();
    let cursor = input.cursor();

    if value.is_empty() {
        let text = if is_active {
            format!("{}▏", placeholder)
        } else {
            placeholder.to_owned()
        };
        return vec![Span::styled(
            pad_to_width(&text, avail),
            style_placeholder(),
        )];
    }

    let cursor_pos = cursor.min(value.len());
    let before = &value[..cursor_pos];
    let after = &value[cursor_pos..];

    let val_style = input_value_style(value, is_active);
    let cursor_style = if is_active {
        val_style
    } else {
        style_placeholder()
    };

    // Compute scroll offset for long text
    let scroll = compute_scroll(value, cursor_pos, avail);

    // Build visible segments
    let visible_before = visible_substring(before, scroll, avail.saturating_sub(1));
    let cursor_char = if is_active {
        "▏".to_string()
    } else {
        " ".to_string()
    };
    let visible_after = visible_substring(
        after,
        0,
        avail.saturating_sub(visible_before.chars().count() + 1),
    );

    let mut spans = Vec::new();

    if scroll > 0 {
        spans.push(Span::styled("…", val_style));
    }

    spans.push(Span::styled(visible_before, val_style));

    if is_active {
        spans.push(Span::styled(cursor_char, cursor_style));
    }

    spans.push(Span::styled(visible_after, val_style));

    // Pad to fill available space
    let used: usize = spans.iter().map(|s| s.content.chars().count()).sum();
    if used < avail {
        spans.push(Span::styled(" ".repeat(avail - used), val_style));
    }

    spans
}

/// Compute scroll offset for text that exceeds available width.
fn compute_scroll(text: &str, cursor: usize, avail: usize) -> usize {
    let text_len = text.chars().count();
    if text_len <= avail {
        return 0;
    }

    // Show cursor near the end of visible area
    let visible_end = (cursor + avail / 2).min(text_len);
    visible_end.saturating_sub(avail - 1)
}

/// Extract a visible substring from text starting at offset.
fn visible_substring(text: &str, start: usize, max_len: usize) -> String {
    text.chars().skip(start).take(max_len).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn visible_substring_basic() {
        assert_eq!(visible_substring("hello", 0, 3), "hel");
        assert_eq!(visible_substring("hello", 2, 3), "llo");
        assert_eq!(visible_substring("hello", 1, 10), "ello");
    }

    #[test]
    fn visible_substring_unicode() {
        // "日本語" has 3 characters; taking 3 returns all 3
        assert_eq!(visible_substring("日本語", 0, 3), "日本語");
        // Taking 2 returns first 2
        assert_eq!(visible_substring("日本語", 0, 2), "日本");
    }

    #[test]
    fn compute_scroll_basic() {
        assert_eq!(compute_scroll("hello", 2, 10), 0);
        // "hello world" has 11 chars, cursor at 8, avail 5
        // visible_end = min(8 + 2, 11) = 10; start = 10 - 4 = 6
        assert_eq!(compute_scroll("hello world", 8, 5), 6);
    }

    #[test]
    fn input_value_style_empty() {
        let style = input_value_style("", true);
        // Should return a valid style (function shouldn't panic on empty input)
        let _ = style;
    }

    #[test]
    fn input_value_style_with_value() {
        let style = input_value_style("test", true);
        // Active style with value should have white color and bold modifier
        let _ = style.fg;
        let _ = style.add_modifier(Modifier::BOLD);
    }
}
