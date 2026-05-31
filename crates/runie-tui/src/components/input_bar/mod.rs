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
use crate::theme::ThemeWrapper;

pub mod builder;
pub use builder::*;

/// Prompt character for the input bar
pub const INPUT_PROMPT: &str = crate::glyphs::CHEVRON_WITH_SPACE;

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
    let border_color: Color = theme.color("border.unfocused").into();
    let accent_color: Color = theme.color("accent.primary").into();

    let block = build_input_block(area, right_info, border_color);
    let inner = block.inner(area);
    block.render(area, buf);

    render_textarea_content(textarea, prompt, inner, accent_color, buf);
}

fn build_input_block(area: Rect, right_info: &str, border_color: Color) -> Block<'static> {
    let mut block = Block::bordered()
        .border_style(Style::default().fg(border_color));

    // Only show right info in border if explicitly provided (e.g. error/status)
    // Model name lives in global_tags, not here
    if !right_info.is_empty() {
        let info_len = right_info.chars().count() as u16;
        let dash_count = area.width.saturating_sub(info_len + 5);
        let dash_str = "─".repeat(dash_count as usize);
        let title_bottom = format!("{} {} {}", dash_str, right_info, "─");
        block = block.title_bottom(Line::from(title_bottom).style(Style::default().fg(border_color)));
    }

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


