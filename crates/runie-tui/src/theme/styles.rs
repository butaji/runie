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

/// Resolve the user-card band color: the theme's `bg.user` token when it
/// exists, otherwise a shade derived from `bg.base` — slightly darker on
/// light themes, slightly lighter on dark themes — so builtin themes
/// without the token get a visible, theme-appropriate band instead of the
/// opaline missing-token fallback gray.
pub(crate) fn bg_user_color(theme: &opaline::Theme) -> opaline::OpalineColor {
    if let Some(c) = theme.try_color("bg.user") {
        return c;
    }
    let base = theme
        .try_color("bg.base")
        .unwrap_or(opaline::OpalineColor::FALLBACK);
    let lum = 0.299 * f32::from(base.r) + 0.587 * f32::from(base.g) + 0.114 * f32::from(base.b);
    if lum > 128.0 {
        base.darken(0.06)
    } else {
        base.lighten(0.06)
    }
}

fn register_chat_styles(theme: &mut opaline::Theme) {
    let accent = theme.color("accent.primary");
    let fg = theme.color("text.primary");
    let dim = theme.color("text.dim");
    let bg_user = bg_user_color(theme);
    // Feed tokens (grok parity): present in the dark runie theme; other
    // themes fall back to their own dim/muted equivalents so light mode
    // keeps its hues while sharing the same structural roles.
    let feed_dim = theme.try_color("feed.dim").unwrap_or(dim);
    // The feed chevron uses the accent color, matching the input-box
    // chevron; weight stays normal so the feed reads calmer than the prompt.
    theme.register_default_style(
        "runie.user",
        opaline::OpalineStyle::fg(accent).with_bg(bg_user),
    );
    theme.register_default_style(
        "runie.user.chevron",
        opaline::OpalineStyle::fg(accent).bold(),
    );
    theme.register_default_style("runie.agent", opaline::OpalineStyle::fg(fg));
    theme.register_default_style("runie.thought", opaline::OpalineStyle::fg(feed_dim));
    theme.register_default_style("runie.thinking", opaline::OpalineStyle::fg(feed_dim));
    theme.register_default_style("runie.thought.summary", opaline::OpalineStyle::fg(feed_dim));
    theme.register_default_style("runie.empty", opaline::OpalineStyle::fg(dim));
    // Status-bar timestamp (out of feed scope) keeps the legacy dim shade;
    // feed timestamps use the grok-parity dim.
    theme.register_default_style("runie.timestamp", opaline::OpalineStyle::fg(dim));
    theme.register_default_style("runie.feed.timestamp", opaline::OpalineStyle::fg(feed_dim));
}

fn register_tool_styles(theme: &mut opaline::Theme) {
    let dim = theme.color("text.dim");
    let feed_dim = theme.try_color("feed.dim").unwrap_or(dim);
    theme.register_default_style("runie.tool.running", opaline::OpalineStyle::fg(feed_dim));
    theme.register_default_style("runie.tool.header", opaline::OpalineStyle::fg(feed_dim));
    theme.register_default_style("runie.tool.output", opaline::OpalineStyle::fg(feed_dim));
    theme.register_default_style("runie.tool.summary", opaline::OpalineStyle::fg(feed_dim));
    theme.register_default_style("runie.turn.complete", opaline::OpalineStyle::fg(feed_dim));
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

// ─────────────────────────────────────────────────────────────────────────────
// Style accessor functions
// ─────────────────────────────────────────────────────────────────────────────

fn style_fn(token: &str) -> Style {
    Style::from(crate::theme::current_theme().style(token))
}

pub fn style_user() -> Style {
    style_fn("runie.user")
}
pub fn style_agent() -> Style {
    style_fn("runie.agent")
}
pub fn style_thought() -> Style {
    style_fn("runie.thought")
}
pub fn style_thinking() -> Style {
    style_fn("runie.thinking")
}
pub fn style_tool_running() -> Style {
    style_fn("runie.tool.running")
}
pub fn style_tool_header() -> Style {
    style_fn("runie.tool.header")
}
pub fn style_tool_output() -> Style {
    style_fn("runie.tool.output")
}
pub fn style_tool_summary() -> Style {
    style_fn("runie.tool.summary")
}
pub fn style_turn_complete() -> Style {
    style_fn("runie.turn.complete")
}
pub fn style_empty_state() -> Style {
    style_fn("runie.empty")
}
pub fn style_timestamp() -> Style {
    style_fn("runie.timestamp")
}
/// Feed timestamp style (grok parity dim) — distinct from the status-bar
/// timestamp, which keeps the legacy `text.dim` shade.
pub fn style_feed_timestamp() -> Style {
    style_fn("runie.feed.timestamp")
}
pub fn style_status_idle() -> Style {
    style_fn("runie.status.idle")
}
pub fn style_status_active() -> Style {
    style_fn("runie.status.active")
}
pub fn style_border() -> Style {
    style_fn("runie.border")
}
pub fn style_border_flash() -> Style {
    style_fn("runie.border.flash")
}
pub fn style_code_block() -> Style {
    style_fn("runie.code.block")
}
pub fn style_code_header() -> Style {
    style_fn("runie.code.header")
}
pub fn style_input_cursor() -> Style {
    style_fn("runie.input.cursor")
}
pub fn style_input_cursor_disabled() -> Style {
    style_fn("runie.input.cursor.disabled")
}
pub fn style_placeholder() -> Style {
    style_fn("runie.placeholder")
}
pub fn style_hint() -> Style {
    style_fn("runie.hint")
}
pub fn style_hint_key() -> Style {
    style_fn("runie.hint.key")
}
pub fn style_popup_selected() -> Style {
    style_fn("runie.popup.selected")
}
pub fn style_popup_unselected() -> Style {
    style_fn("runie.popup.unselected")
}
pub fn style_popup_border() -> Style {
    style_fn("runie.popup.border")
}
pub fn style_thought_summary() -> Style {
    style_fn("runie.thought.summary")
}
pub fn style_diff_insert() -> Style {
    style_fn("runie.diff.insert")
}
pub fn style_diff_remove() -> Style {
    style_fn("runie.diff.remove")
}
pub fn style_diff_hunk() -> Style {
    style_fn("runie.diff.hunk")
}
pub fn style_diff_file_header() -> Style {
    style_fn("runie.diff.file_header")
}
pub fn style_diff_context() -> Style {
    style_fn("runie.diff.context")
}

/// Scrollbar style: visible but subtle — dimmed text color on app bg.
/// Shows a clear thumb without being distracting.
pub fn style_scrollbar() -> Style {
    Style::default().fg(crate::theme::color_dim())
}

/// Chevron style for input box: orange when input holds the token, gray when released.
/// Uses style without background for the input box.
pub fn style_chevron(token_held: bool) -> Style {
    if token_held {
        style_fn("runie.user.chevron")
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
