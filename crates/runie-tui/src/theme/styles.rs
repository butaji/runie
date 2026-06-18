use ratatui::layout::Alignment;
use ratatui::style::Style;
use ratatui::text::Line;
use ratatui::widgets::{Block, BorderType, Borders, TitlePosition};

pub(crate) fn register_runie_styles(mut theme: opaline::Theme) -> opaline::Theme {
    register_chat_styles(&mut theme);
    register_tool_styles(&mut theme);
    register_status_styles(&mut theme);
    register_code_styles(&mut theme);
    register_input_styles(&mut theme);
    register_diff_styles(&mut theme);
    register_runie_popup_styles(&mut theme);
    theme
}

fn register_chat_styles(theme: &mut opaline::Theme) {
    let accent = theme.color("accent.primary");
    let fg = theme.color("text.primary");
    let dim = theme.color("text.dim");
    theme.register_default_style("runie.user", opaline::OpalineStyle::fg(accent).bold());
    theme.register_default_style("runie.agent", opaline::OpalineStyle::fg(fg));
    theme.register_default_style("runie.thought", opaline::OpalineStyle::fg(dim));
    theme.register_default_style("runie.thinking", opaline::OpalineStyle::fg(dim));
    theme.register_default_style("runie.thought.summary", opaline::OpalineStyle::fg(dim));
    theme.register_default_style("runie.empty", opaline::OpalineStyle::fg(dim));
    theme.register_default_style("runie.timestamp", opaline::OpalineStyle::fg(dim));
}

fn register_tool_styles(theme: &mut opaline::Theme) {
    let dim = theme.color("text.dim");
    let fg = theme.color("text.primary");
    theme.register_default_style("runie.tool.running", opaline::OpalineStyle::fg(dim));
    theme.register_default_style("runie.tool.header", opaline::OpalineStyle::fg(dim));
    theme.register_default_style("runie.tool.output", opaline::OpalineStyle::fg(fg));
    theme.register_default_style("runie.tool.summary", opaline::OpalineStyle::fg(dim));
    theme.register_default_style("runie.turn.complete", opaline::OpalineStyle::fg(dim));
}

fn register_status_styles(theme: &mut opaline::Theme) {
    let dim = theme.color("text.dim");
    let success = theme.color("success");
    let warning = theme.color("warning");
    theme.register_default_style("runie.status.idle", opaline::OpalineStyle::fg(dim));
    theme.register_default_style("runie.status.active", opaline::OpalineStyle::fg(success));
    theme.register_default_style(
        "runie.border",
        opaline::OpalineStyle::fg(theme.color("border.unfocused")),
    );
    theme.register_default_style("runie.border.flash", opaline::OpalineStyle::fg(warning));
}

fn register_code_styles(theme: &mut opaline::Theme) {
    let dim = theme.color("text.dim");
    let code_fn = theme.color("code.function");
    let bg_code = theme.color("bg.code");
    theme.register_default_style(
        "runie.code.block",
        opaline::OpalineStyle::fg(code_fn).with_bg(bg_code),
    );
    theme.register_default_style("runie.code.header", opaline::OpalineStyle::fg(dim));
}

fn register_diff_styles(theme: &mut opaline::Theme) {
    let success = theme.color("success");
    let error = theme.color("error");
    let bg = theme.color("bg.base");
    let dim = theme.color("text.dim");

    // Insert: green text on a subtle green-tinted background.
    let insert_bg = blend_opaline(bg, success, 0.12);
    theme.register_default_style(
        "runie.diff.insert",
        opaline::OpalineStyle::fg(success).with_bg(insert_bg),
    );
    // Remove: red text on a subtle red-tinted background.
    let remove_bg = blend_opaline(bg, error, 0.12);
    theme.register_default_style(
        "runie.diff.remove",
        opaline::OpalineStyle::fg(error).with_bg(remove_bg),
    );
    // Hunk header: accent foreground, bold.
    theme.register_default_style(
        "runie.diff.hunk",
        opaline::OpalineStyle::fg(theme.color("accent.primary")).bold(),
    );
    // File header: dim text.
    theme.register_default_style("runie.diff.file_header", opaline::OpalineStyle::fg(dim));
    // Context: plain foreground.
    theme.register_default_style(
        "runie.diff.context",
        opaline::OpalineStyle::fg(theme.color("text.primary")),
    );
}

/// Blend two opaline colors at the given opacity (0.0–1.0).
fn blend_opaline(
    bg: opaline::OpalineColor,
    fg: opaline::OpalineColor,
    opacity: f32,
) -> opaline::OpalineColor {
    bg.lerp(fg, opacity)
}

fn register_input_styles(theme: &mut opaline::Theme) {
    let dim = theme.color("text.dim");
    let cursor_fg = theme
        .try_color("bg.base")
        .unwrap_or_else(|| theme.color("text.primary"));
    let accent = theme.color("accent.primary");
    theme.register_default_style(
        "runie.input.cursor",
        opaline::OpalineStyle::fg(cursor_fg).with_bg(accent),
    );
    // Disabled cursor (vim nav mode): gray block, no accent.
    theme.register_default_style(
        "runie.input.cursor.disabled",
        opaline::OpalineStyle::fg(dim).with_bg(theme.color("text.muted")),
    );
    theme.register_default_style("runie.placeholder", opaline::OpalineStyle::fg(dim));
    theme.register_default_style("runie.hint", opaline::OpalineStyle::fg(dim));
    theme.register_default_style(
        "runie.hint.key",
        opaline::OpalineStyle::fg(theme.color("text.muted")),
    );
}

fn register_runie_popup_styles(theme: &mut opaline::Theme) {
    let fg_secondary = theme.color("text.secondary");
    let border_focused = theme.color("border.focused");
    let accent_secondary = theme.color("accent.secondary");
    let bg_highlight = theme.color("bg.highlight");

    theme.register_default_style(
        "runie.popup.selected",
        opaline::OpalineStyle::fg(accent_secondary)
            .with_bg(bg_highlight)
            .bold(),
    );
    theme.register_default_style(
        "runie.popup.unselected",
        opaline::OpalineStyle::fg(fg_secondary),
    );
    theme.register_default_style(
        "runie.popup.border",
        opaline::OpalineStyle::fg(border_focused),
    );
}

macro_rules! style_fn {
    ($name:ident, $token:literal) => {
        pub fn $name() -> Style {
            Style::from(crate::theme::current_theme().style($token))
        }
    };
}

style_fn!(style_user, "runie.user");
style_fn!(style_agent, "runie.agent");
style_fn!(style_thought, "runie.thought");
style_fn!(style_thinking, "runie.thinking");
style_fn!(style_tool_running, "runie.tool.running");
style_fn!(style_tool_header, "runie.tool.header");
style_fn!(style_tool_output, "runie.tool.output");
style_fn!(style_tool_summary, "runie.tool.summary");
style_fn!(style_turn_complete, "runie.turn.complete");
style_fn!(style_empty_state, "runie.empty");
style_fn!(style_timestamp, "runie.timestamp");
style_fn!(style_status_idle, "runie.status.idle");
style_fn!(style_status_active, "runie.status.active");
style_fn!(style_border, "runie.border");
style_fn!(style_border_flash, "runie.border.flash");
style_fn!(style_code_block, "runie.code.block");
style_fn!(style_code_header, "runie.code.header");
style_fn!(style_input_cursor, "runie.input.cursor");
style_fn!(style_input_cursor_disabled, "runie.input.cursor.disabled");
style_fn!(style_placeholder, "runie.placeholder");
style_fn!(style_hint, "runie.hint");
style_fn!(style_hint_key, "runie.hint.key");
style_fn!(style_popup_selected, "runie.popup.selected");
style_fn!(style_popup_unselected, "runie.popup.unselected");
style_fn!(style_popup_border, "runie.popup.border");
style_fn!(style_thought_summary, "runie.thought.summary");
style_fn!(style_diff_insert, "runie.diff.insert");
style_fn!(style_diff_remove, "runie.diff.remove");
style_fn!(style_diff_hunk, "runie.diff.hunk");
style_fn!(style_diff_file_header, "runie.diff.file_header");
style_fn!(style_diff_context, "runie.diff.context");

/// Scrollbar style: visible but subtle — dimmed text color on app bg.
/// Shows a clear thumb without being distracting.
pub fn style_scrollbar() -> Style {
    Style::default().fg(crate::theme::color_dim())
}

/// Chevron style: orange when input holds the token, gray when released.
pub fn style_chevron(token_held: bool) -> Style {
    if token_held {
        style_user()
    } else {
        style_hint()
    }
}

/// Build the input panel block with rounded borders.
/// Title is placed at the bottom-right.
pub fn block_input(title: &str, flash: bool) -> Block<'_> {
    let border_style = if flash {
        style_border_flash()
    } else {
        style_border()
    };
    Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .title_position(TitlePosition::Bottom)
        .title(Line::from(title).alignment(Alignment::Right))
        .border_style(border_style)
}

/// Build a popup dialog block with rounded borders.
pub fn block_popup(title: &str) -> Block<'_> {
    Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .title(title)
        .border_style(style_popup_border())
        .style(Style::default().bg(crate::theme::color_bg_panel()))
}
