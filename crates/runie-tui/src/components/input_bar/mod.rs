//! Input bar component using ratatui-textarea.
//!
//! The TextArea handles all text editing (insert, delete, cursor movement, etc.)
//! via its `input()` method which processes crossterm Events.

use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Style},
    prelude::Widget,
    text::{Line, Span},
    widgets::Block,
};
use crate::theme::ThemeColors;

pub mod builder;
pub use builder::*;

/// Prompt character for the input bar
pub const INPUT_PROMPT: &str = crate::glyphs::CHEVRON_WITH_SPACE;

/// Calculate the height needed for the input bar based on textarea content.
pub fn input_bar_height(textarea: &ratatui_textarea::TextArea, has_attachments: bool) -> u16 {
    let visual_lines = textarea.lines().len().max(1);
    let base_height = (visual_lines as u16) + 2; // +2 for borders
    if has_attachments { base_height + 1 } else { base_height }
}

/// Render the input bar with prompt and right info.
///
/// The textarea is rendered as a widget with cursor styled in accent color.
/// The prompt is overlaid on the first line of the textarea content.
pub fn render_input_bar(
    textarea: &ratatui_textarea::TextArea,
    prompt: &str,
    right_info: &str,
    area: Rect,
    buf: &mut Buffer,
    colors: &ThemeColors,
    mode_indicator: &str,
    attached_files: &[String],
    char_count: Option<usize>,
    is_focused: bool,
) {
    let border_color = if is_focused {
        colors.accent_primary
    } else {
        colors.border_unfocused
    };

    // Build mode indicator style
    let mode_style = if mode_indicator.contains("plan") {
        Style::default().fg(colors.text_plan)
    } else if mode_indicator.contains("yolo") {
        Style::default().fg(colors.accent_primary)
    } else {
        Style::default().fg(colors.text_dim)
    };

    let block = build_input_block(area, right_info, border_color, mode_indicator, mode_style, char_count, colors);
    let inner = block.inner(area);
    block.render(area, buf);

    let text = textarea.lines().join("");
    let is_empty = text.trim().is_empty();

    if is_empty && !is_focused {
        // Show placeholder when empty and not focused
        render_placeholder(placeholder_text(), inner, colors, buf);
    }
    // Always render textarea (provides cursor when focused, even when empty)
    render_textarea_content(textarea, prompt, inner, colors.accent_primary, buf);
    render_attachments(area, buf, attached_files, colors);
}

/// Returns the placeholder text to display when input is empty
fn placeholder_text() -> &'static str {
    "Build anything..."
}

fn build_input_block(
    area: Rect,
    right_info: &str,
    border_color: Color,
    mode_indicator: &str,
    _mode_style: Style,
    char_count: Option<usize>,
    colors: &ThemeColors,
) -> Block<'static> {
    let mut block = Block::bordered()
        .border_set(ratatui::symbols::border::ROUNDED)
        .border_style(Style::default().fg(border_color));

    // Build bottom title with mode indicator and char count
    let mode_len = mode_indicator.chars().count() as u16;
    let count_str = char_count.map(|c| format!(" · {} chars", c)).unwrap_or_default();
    let count_len = count_str.chars().count() as u16;

    let right_info_len = if right_info.is_empty() {
        0
    } else {
        right_info.chars().count() as u16 + 2
    };

    let dash_count = area.width.saturating_sub(mode_len + count_len + right_info_len + 5);
    let dash_str = "─".repeat(dash_count as usize);

    let title_bottom = if right_info.is_empty() {
        format!("{} {} {}{}", dash_str, mode_indicator, count_str, "─")
    } else {
        format!("{} {} {}{} {} {}", dash_str, mode_indicator, count_str, "─", right_info, "─")
    };

    block = block.title_bottom(
        Line::from(title_bottom)
            .style(Style::default().fg(colors.border_unfocused))
    );

    block
}

fn render_textarea_content(textarea: &ratatui_textarea::TextArea, prompt: &str, inner: Rect, accent_color: Color, buf: &mut Buffer) {
    let prompt_width = prompt.chars().count() as u16;
    let text_area = Rect {
        x: inner.x + prompt_width,
        y: inner.y,
        width: inner.width.saturating_sub(prompt_width),
        height: inner.height,
    };
    Widget::render(textarea, text_area, buf);

    let prompt_span = Span::styled(prompt, Style::default().fg(accent_color));
    let prompt_line = Line::from(vec![prompt_span]);
    buf.set_line(inner.x, inner.y, &prompt_line, prompt_width);
}

/// Render placeholder text when input is empty and unfocused.
fn render_placeholder(text: &str, inner: Rect, colors: &ThemeColors, buf: &mut Buffer) {
    let placeholder_style = Style::default().fg(colors.text_dim);
    let placeholder_line = Line::styled(text, placeholder_style);
    buf.set_line(inner.x + 1, inner.y, &placeholder_line, inner.width.saturating_sub(2));
}

/// Render attached file pills below the input bar.
fn render_attachments(area: Rect, buf: &mut Buffer, files: &[String], colors: &ThemeColors) {
    if files.is_empty() {
        return;
    }

    let y = area.bottom().saturating_sub(1);
    let mut x = area.x + 2;

    for file in files.iter().take(5) {
        let pill = format!("📄 {} ", file);
        let pill_style = Style::default()
            .fg(colors.text_dim);
        buf.set_string(x, y, &pill, pill_style);
        x += pill.chars().count() as u16;
    }
}
