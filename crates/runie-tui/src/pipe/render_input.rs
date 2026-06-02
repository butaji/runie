//! Pipe render_input module.

use ratatui::{buffer::Buffer, layout::Rect, style::Style};
use crate::tui::AppState;
use crate::theme::ThemeWrapper;
use crate::theme::ThemeColors;

pub fn render_input(
    buf: &mut Buffer,
    state: &AppState,
    area: Rect,
    theme: &ThemeWrapper,
    theme_colors: &ThemeColors,
) {
    use crate::tui::state::PermissionMode;

    let mut textarea = state.textarea.clone();
    let accent_color = theme.color("accent.primary").into();
    let text_primary = theme.color("text.primary").into();
    textarea.set_style(Style::default().fg(text_primary));
    textarea.set_cursor_style(Style::default().fg(accent_color).bg(accent_color));
    textarea.set_cursor_line_style(Style::default().remove_modifier(ratatui::style::Modifier::UNDERLINED));
    let text = state.textarea.lines().join("\n");
    let prompt = if text.starts_with('!') {
        "! ".to_string()
    } else if text.starts_with('@') {
        "@ ".to_string()
    } else {
        format!(" {ch} ", ch = crate::glyphs::CHEVRON)
    };

    // Build mode indicator
    let mode_indicator = match state.permission_mode {
        PermissionMode::Normal => "runie".to_string(),
        PermissionMode::Plan => "runie · plan".to_string(),
        PermissionMode::AutoApprove => "runie · yolo".to_string(),
    };

    // Calculate char count if text is long (>50% of context window)
    let char_count = {
        let text_len = text.len();
        let ctx_window = state.top_bar.context_window.unwrap_or(128_000);
        let estimated_tokens = text_len * 4;
        if estimated_tokens > ctx_window / 2 {
            Some(text_len)
        } else {
            None
        }
    };

    // Attached files (empty in pipe render path for now)
    let attached_files: Vec<String> = Vec::new();

    crate::components::input_bar::render_input_bar(
        &textarea,
        &prompt,
        &state.input_right_info,
        area,
        buf,
        theme_colors,
        &mode_indicator,
        &attached_files,
        char_count,
        !state.scroll.scroll_focused, // is_focused
    );
}
