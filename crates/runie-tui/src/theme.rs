//! Theme system powered by opaline
//!
//! Runie-specific styles are registered as defaults so any theme can override them.
//! The current theme is cached in a global lock; `draw_snapshot` sets it at frame start.

use ratatui::layout::Alignment;
use ratatui::style::{Color, Style};
use ratatui::text::Line;
use ratatui::widgets::{Block, BorderType, Borders, TitlePosition};
use std::sync::{Arc, RwLock};

static CURRENT_THEME: RwLock<Option<Arc<opaline::Theme>>> = RwLock::new(None);

#[cfg(test)]
pub(crate) static TEST_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

#[cfg(test)]
pub fn test_lock() -> std::sync::MutexGuard<'static, ()> {
    TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner())
}

/// Set the active theme by name. Called by `draw_snapshot` at frame start.
pub fn set_current_theme(name: &str) {
    let theme = load_theme(name);
    let mut guard = CURRENT_THEME.write().unwrap_or_else(|e| e.into_inner());
    *guard = Some(Arc::new(theme));
}

pub const DEFAULT_THEME_NAME: &str = "runie";

/// Get the currently active theme (falls back to default).
pub fn current_theme() -> Arc<opaline::Theme> {
    let guard = CURRENT_THEME.read().unwrap_or_else(|e| e.into_inner());
    guard.clone().unwrap_or_else(|| Arc::new(default_theme()))
}

/// Load a theme by name: builtin → custom file → default fallback.
fn load_theme(name: &str) -> opaline::Theme {
    if let Some(theme) = opaline::load_by_name(name) {
        return register_runie_styles(theme);
    }
    let custom_path = dirs::home_dir()
        .unwrap_or_else(|| std::path::PathBuf::from("."))
        .join(".runie")
        .join("themes")
        .join(format!("{}.toml", name));
    if let Ok(theme) = opaline::load_from_file(&custom_path) {
        return register_runie_styles(theme);
    }
    register_runie_styles(default_theme())
}

/// List all available builtin theme names.
pub fn list_builtin_themes() -> Vec<&'static str> {
    runie_core::themes::BUILTIN_THEMES.to_vec()
}

const DEFAULT_THEME_TOML: &str = r##"
[meta]
name = "Runie"
author = "runie"
variant = "dark"
version = "1.0"
description = "Dark base with vibrant orange accents"

[palette]
orange_500 = "#EE6902"
orange_400 = "#F5853F"
orange_600 = "#D45A00"
orange_300 = "#F9A85F"
amber_400 = "#F5A623"
amber_300 = "#F9C846"
coral_400 = "#E85577"
green_400 = "#4ADE80"
red_400 = "#EF4444"
blue_400 = "#60A5FA"
lime_400 = "#A3E635"

bg_code = "#1E1920"
bg_highlight = "#2A202E"
bg_elevated = "#201820"
bg_active = "#302830"
bg_selection = "#3A2E38"

text_primary = "#EDE8E3"
text_secondary = "#C4BEB7"
text_muted = "#8A8580"
text_dim = "#5C5854"

[tokens]
"text.primary" = "text_primary"
"text.secondary" = "text_secondary"
"text.muted" = "text_muted"
"text.dim" = "text_dim"

"bg.code" = "bg_code"
"bg.highlight" = "bg_highlight"
"bg.elevated" = "bg_elevated"
"bg.active" = "bg_active"
"bg.selection" = "bg_selection"
"bg.user" = "bg_elevated"

"accent.primary" = "orange_500"
"accent.secondary" = "amber_400"
"accent.tertiary" = "coral_400"
"accent.deep" = "orange_600"

success = "green_400"
error = "red_400"
warning = "amber_400"
info = "blue_400"

"border.focused" = "orange_500"
"border.unfocused" = "text_dim"

"code.keyword" = "amber_400"
"code.function" = "orange_500"
"code.string" = "lime_400"
"code.number" = "amber_300"
"code.comment" = "text_dim"
"code.type" = "blue_400"
"code.line_number" = "text_dim"

[styles]
keyword = { fg = "accent.primary", bold = true }
line_number = { fg = "code.line_number" }
cursor_line = { bg = "bg.highlight" }
selected = { fg = "accent.secondary", bg = "bg.highlight" }
active_selected = { fg = "accent.primary", bg = "bg.active", bold = true }
focused_border = { fg = "border.focused" }
unfocused_border = { fg = "border.unfocused" }
success_style = { fg = "success" }
error_style = { fg = "error" }
warning_style = { fg = "warning" }
info_style = { fg = "info" }
dimmed = { fg = "text.dim" }
muted = { fg = "text.muted" }
inline_code = { fg = "success", bg = "bg.code" }
"##;

fn default_theme() -> opaline::Theme {
    opaline::load_from_str(DEFAULT_THEME_TOML, None).expect("embedded default theme must be valid")
}

fn register_runie_styles(mut theme: opaline::Theme) -> opaline::Theme {
    register_chat_styles(&mut theme);
    register_tool_styles(&mut theme);
    register_status_styles(&mut theme);
    register_code_styles(&mut theme);
    register_input_styles(&mut theme);
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

// ── Raw color helpers (for markdown, diff, etc.) ───────────────────────

pub fn color_bg() -> Color {
    current_theme()
        .try_color("bg.base")
        .map(Color::from)
        .unwrap_or(Color::Reset)
}
pub fn color_bg_panel() -> Color {
    current_theme()
        .try_color("bg.panel")
        .map(Color::from)
        .unwrap_or(Color::Reset)
}
pub fn color_fg() -> Color {
    Color::from(current_theme().color("text.primary"))
}
pub fn color_fg_mid() -> Color {
    Color::from(current_theme().color("text.secondary"))
}
pub fn color_fg_bright() -> Color {
    let c = current_theme().color("text.primary").lighten(0.3);
    Color::Rgb(c.r, c.g, c.b)
}
pub fn color_accent() -> Color {
    Color::from(current_theme().color("accent.primary"))
}
pub fn color_success() -> Color {
    Color::from(current_theme().color("success"))
}
pub fn color_warning() -> Color {
    Color::from(current_theme().color("warning"))
}
pub fn color_error() -> Color {
    Color::from(current_theme().color("error"))
}
pub fn color_dim() -> Color {
    Color::from(current_theme().color("text.dim"))
}
pub fn color_border() -> Color {
    Color::from(current_theme().color("border.unfocused"))
}

/// Darken an RGB color by a factor (0.0–1.0).
pub fn darken(color: Color, factor: f32) -> Color {
    match color {
        Color::Rgb(r, g, b) => Color::Rgb(
            (r as f32 * factor).clamp(0.0, 255.0) as u8,
            (g as f32 * factor).clamp(0.0, 255.0) as u8,
            (b as f32 * factor).clamp(0.0, 255.0) as u8,
        ),
        _ => color,
    }
}
pub fn color_code() -> Color {
    Color::from(current_theme().color("code.function"))
}
pub fn color_code_bg() -> Color {
    current_theme()
        .try_color("bg.code")
        .map(Color::from)
        .unwrap_or(Color::Reset)
}

/// User message post background. Themes can override `bg.user`;
/// otherwise we fall back to the elevated surface color.
pub fn color_user_bg() -> Color {
    current_theme()
        .try_color("bg.user")
        .or_else(|| current_theme().try_color("bg.elevated"))
        .or_else(|| current_theme().try_color("bg.panel"))
        .or_else(|| current_theme().try_color("bg.highlight"))
        .map(Color::from)
        .unwrap_or(Color::Reset)
}

/// Accent color blended over the terminal background at the given
/// opacity (0.0–1.0). Used for the subtle selection highlight behind
/// the selected post in vim nav mode.
pub fn color_accent_bg() -> Color {
    blend(color_bg(), color_accent(), 0.1)
}

fn blend(bg: Color, fg: Color, opacity: f32) -> Color {
    let opacity = opacity.clamp(0.0, 1.0);
    let (br, bg_g, bb) = match bg {
        Color::Rgb(r, g, b) => (r, g, b),
        _ => (30, 30, 30),
    };
    let (fr, fg_g, fb) = match fg {
        Color::Rgb(r, g, b) => (r, g, b),
        _ => return fg,
    };
    Color::Rgb(
        (br as f32 * (1.0 - opacity) + fr as f32 * opacity) as u8,
        (bg_g as f32 * (1.0 - opacity) + fg_g as f32 * opacity) as u8,
        (bb as f32 * (1.0 - opacity) + fb as f32 * opacity) as u8,
    )
}

// ── Style helpers ──────────────────────────────────────────────────────

macro_rules! style_fn {
    ($name:ident, $token:literal) => {
        pub fn $name() -> Style {
            Style::from(current_theme().style($token))
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

/// Scrollbar style: visible but subtle — dimmed text color on app bg.
/// Shows a clear thumb without being distracting.
pub fn style_scrollbar() -> Style {
    Style::default().fg(color_dim())
}

/// Chevron style: orange when input holds the token, gray when released.
pub fn style_chevron(token_held: bool) -> Style {
    if token_held {
        style_user()
    } else {
        style_hint()
    }
}

// ── Block helpers — centralized border styling ─────────────────────────

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
        .style(Style::default().bg(color_bg_panel()))
}

// ── Glyphs and formatting helpers (unchanged) ──────────────────────────

pub const GLYPH_USER: &str = "❯ ";
pub const GLYPH_AGENT: &str = "→ ";
pub const GLYPH_TOOL: &str = "✓ ";
pub const GLYPH_INDENT: &str = "  ";
pub const GLYPH_SELECTED: &str = "▸ ";
pub const GLYPH_UNSELECTED: &str = "  ";
pub const GLYPH_THINKING: char = '◐';
pub const GLYPH_SPINNER: char = '⠋';
pub const SCROLLBAR_TRACK: &str = " "; // invisible track
pub const SCROLLBAR_THUMB: &str = "▐"; // right half-block — visible but not heavy
pub const INDICATOR_COLLAPSED: &str = " [+]";
pub const PANEL_CHAT: &str = " Chat ";
pub const PANEL_INPUT: &str = " Input ";

pub fn code_header_label(prefix: &str, lang: &str) -> String {
    if lang.is_empty() {
        format!("{}[code]", prefix)
    } else {
        format!("{}[code:{}]", prefix, lang)
    }
}

pub fn thinking_line(elapsed_secs: f64) -> String {
    format!("{} {} {:.1}s", GLYPH_AGENT, GLYPH_THINKING, elapsed_secs)
}

pub fn tool_running_line(name: &str, elapsed_secs: f64) -> String {
    format!("{}Running {}... {:.1}s", GLYPH_TOOL, name, elapsed_secs)
}

pub fn tool_done_header(name: &str, duration_secs: f64) -> String {
    format!("{}{} {:.1}s", GLYPH_TOOL, name, duration_secs)
}

pub fn tool_summary_line(name: &str, duration_secs: f64) -> String {
    format!(
        "{}{} {:.1}s{}",
        GLYPH_TOOL, name, duration_secs, INDICATOR_COLLAPSED
    )
}

pub fn turn_complete_line(duration_secs: f64) -> String {
    format!("Turn completed in {:.1}s", duration_secs)
}

pub fn thought_summary_line(first_line: &str) -> String {
    format!("{}{}", first_line, INDICATOR_COLLAPSED)
}
