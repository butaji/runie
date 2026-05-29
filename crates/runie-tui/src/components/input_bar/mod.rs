//! Input bar component using ratatui-textarea.
//!
//! The TextArea handles all text editing (insert, delete, cursor movement, etc.)
//! via its `input()` method which processes crossterm Events.

use ratatui::{buffer::Buffer, layout::Rect, style::Color, prelude::Widget};
use crate::theme::ThemeWrapper;

pub mod builder;
pub use builder::*;

/// Prompt character for the input bar
pub const INPUT_PROMPT: &str = "\u{276F} ";

/// Calculate the height needed for the input bar based on textarea content.
pub fn input_bar_height(textarea: &ratatui_textarea::TextArea) -> u16 {
    let visual_lines = textarea.lines().len().max(1);
    (visual_lines as u16) + 2 // +2 for borders
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
    theme: &ThemeWrapper,
) {
    use ratatui::style::Style;
    use ratatui::text::{Line, Span};
    use ratatui::widgets::Block;

    let border_color: Color = theme.color("border.unfocused").into();
    let accent_color: Color = theme.color("accent.primary").into();

    // Build border block with bottom title
    let info_text = if right_info.is_empty() { "model: claude-4" } else { right_info };
    let info_len = info_text.chars().count() as u16;
    let dash_count = area.width.saturating_sub(info_len + 5);
    let dash_str = "─".repeat(dash_count as usize);
    let title_bottom = format!("{} {} {}", dash_str, info_text, "─");

    let block = Block::bordered()
        .border_style(Style::default().fg(border_color))
        .title_bottom(Line::from(title_bottom).style(Style::default().fg(border_color)));

    let inner = block.inner(area);
    block.render(area, buf);

    // Calculate prompt width
    let prompt_width = prompt.chars().count() as u16;

    // Render TextArea shifted right by prompt width.
    // Line 0: prompt at inner.x, text starts after prompt.
    // Line 1+: text indented by prompt_width (aligns with text on line 0).
    let text_area = Rect {
        x: inner.x + prompt_width,
        y: inner.y,
        width: inner.width.saturating_sub(prompt_width),
        height: inner.height,
    };
    // TextArea implements Widget trait directly — render via trait method
    ratatui::widgets::Widget::render(textarea, text_area, buf);

    // Render prompt at start of first line
    let prompt_span = Span::styled(prompt, Style::default().fg(accent_color));
    let prompt_line = Line::from(vec![prompt_span]);
    buf.set_line(inner.x, inner.y, &prompt_line, prompt_width);
}


