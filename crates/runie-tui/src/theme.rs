//! Theme system powered by opaline
//!
//! Runie-specific styles are registered as defaults so any theme can override them.
//! The current theme is cached in a global lock; `draw_snapshot` sets it at frame start.

use ratatui::style::{Color, Style};
use std::sync::{Arc, RwLock};

static CURRENT_THEME: RwLock<Option<Arc<opaline::Theme>>> = RwLock::new(None);

/// Set the active theme by name. Called by `draw_snapshot` at frame start.
pub fn set_current_theme(name: &str) {
    let theme = load_theme(name);
    let mut guard = CURRENT_THEME.write().unwrap_or_else(|e| e.into_inner());
    *guard = Some(Arc::new(theme));
}

/// Get the currently active theme (falls back to default).
pub fn current_theme() -> Arc<opaline::Theme> {
    let guard = CURRENT_THEME.read().unwrap_or_else(|e| e.into_inner());
    guard.clone().unwrap_or_else(|| Arc::new(opaline::Theme::default()))
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
    register_runie_styles(opaline::Theme::default())
}

/// List all available builtin theme names.
pub fn list_builtin_themes() -> Vec<&'static str> {
    vec![
        "silkcircuit-neon", "silkcircuit-glow", "silkcircuit-soft", "silkcircuit-vibrant", "silkcircuit-dawn",
        "catppuccin-mocha", "catppuccin-macchiato", "catppuccin-frappe", "catppuccin-latte",
        "dracula", "nord", "gruvbox-dark", "gruvbox-light", "tokyo-night", "tokyo-night-storm", "tokyo-night-moon",
        "rose-pine", "rose-pine-moon", "rose-pine-dawn", "kanagawa-wave", "kanagawa-dragon", "kanagawa-lotus",
        "everforest-dark", "everforest-light", "ayu-dark", "ayu-light", "ayu-mirage",
        "one-dark", "one-light", "github-dark-dimmed", "github-light", "night-owl", "light-owl",
        "monokai-pro", "palenight", "solarized-dark", "solarized-light", "flexoki-dark", "flexoki-light",
    ]
}

fn register_runie_styles(mut theme: opaline::Theme) -> opaline::Theme {
    let accent = theme.color("accent.primary");
    let fg = theme.color("text.primary");
    let dim = theme.color("text.dim");
    let success = theme.color("success");
    let warning = theme.color("warning");
    let bg = theme.color("bg.base");
    let bg_code = theme.color("bg.code");
    let code_fn = theme.color("code.function");

    theme.register_default_style("runie.user", opaline::OpalineStyle::fg(accent).bold());
    theme.register_default_style("runie.agent", opaline::OpalineStyle::fg(fg));
    theme.register_default_style("runie.thought", opaline::OpalineStyle::fg(dim));
    theme.register_default_style("runie.thinking", opaline::OpalineStyle::fg(dim));
    theme.register_default_style("runie.tool.running", opaline::OpalineStyle::fg(dim));
    theme.register_default_style("runie.tool.header", opaline::OpalineStyle::fg(dim));
    theme.register_default_style("runie.tool.output", opaline::OpalineStyle::fg(fg));
    theme.register_default_style("runie.tool.summary", opaline::OpalineStyle::fg(dim));
    theme.register_default_style("runie.turn.complete", opaline::OpalineStyle::fg(dim));
    theme.register_default_style("runie.empty", opaline::OpalineStyle::fg(dim));
    theme.register_default_style("runie.timestamp", opaline::OpalineStyle::fg(dim));
    theme.register_default_style("runie.status.idle", opaline::OpalineStyle::fg(dim));
    theme.register_default_style("runie.status.active", opaline::OpalineStyle::fg(success));
    theme.register_default_style("runie.border", opaline::OpalineStyle::fg(theme.color("border.unfocused")));
    theme.register_default_style("runie.border.flash", opaline::OpalineStyle::fg(warning));
    theme.register_default_style("runie.code.block", opaline::OpalineStyle::fg(code_fn).with_bg(bg_code));
    theme.register_default_style("runie.code.header", opaline::OpalineStyle::fg(dim));
    theme.register_default_style("runie.input.cursor", opaline::OpalineStyle::fg(bg).with_bg(fg));
    theme.register_default_style("runie.placeholder", opaline::OpalineStyle::fg(dim));
    theme.register_default_style("runie.hint", opaline::OpalineStyle::fg(dim));
    theme.register_default_style("runie.thought.summary", opaline::OpalineStyle::fg(dim));

    register_runie_popup_styles(&mut theme);
    theme
}

fn register_runie_popup_styles(theme: &mut opaline::Theme) {
    let fg_secondary = theme.color("text.secondary");
    let border_focused = theme.color("border.focused");
    let accent_secondary = theme.color("accent.secondary");
    let bg_highlight = theme.color("bg.highlight");

    theme.register_default_style(
        "runie.popup.selected",
        opaline::OpalineStyle::fg(accent_secondary).with_bg(bg_highlight).bold(),
    );
    theme.register_default_style("runie.popup.unselected", opaline::OpalineStyle::fg(fg_secondary));
    theme.register_default_style("runie.popup.border", opaline::OpalineStyle::fg(border_focused));
}

// ── Raw color helpers (for markdown, diff, etc.) ───────────────────────

pub fn color_bg() -> Color { Color::from(current_theme().color("bg.base")) }
pub fn color_fg() -> Color { Color::from(current_theme().color("text.primary")) }
pub fn color_fg_mid() -> Color { Color::from(current_theme().color("text.secondary")) }
pub fn color_fg_bright() -> Color {
    let c = current_theme().color("text.primary").lighten(0.3);
    Color::Rgb(c.r, c.g, c.b)
}
pub fn color_accent() -> Color { Color::from(current_theme().color("accent.primary")) }
pub fn color_success() -> Color { Color::from(current_theme().color("success")) }
pub fn color_warning() -> Color { Color::from(current_theme().color("warning")) }
pub fn color_dim() -> Color { Color::from(current_theme().color("text.dim")) }
pub fn color_code() -> Color { Color::from(current_theme().color("code.function")) }
pub fn color_code_bg() -> Color { Color::from(current_theme().color("bg.code")) }

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
style_fn!(style_placeholder, "runie.placeholder");
style_fn!(style_hint, "runie.hint");
style_fn!(style_popup_selected, "runie.popup.selected");
style_fn!(style_popup_unselected, "runie.popup.unselected");
style_fn!(style_popup_border, "runie.popup.border");
style_fn!(style_thought_summary, "runie.thought.summary");

// ── Glyphs and formatting helpers (unchanged) ──────────────────────────

pub const GLYPH_USER: &str = "$ ";
pub const GLYPH_AGENT: &str = "→ ";
pub const GLYPH_TOOL: &str = "✓ ";
pub const GLYPH_INDENT: &str = "  ";
pub const GLYPH_SELECTED: &str = "▸ ";
pub const GLYPH_UNSELECTED: &str = "  ";
pub const GLYPH_THINKING: char = '◐';
pub const GLYPH_SPINNER: char = '⠋';
pub const SCROLLBAR_TRACK: &str = "│";
pub const SCROLLBAR_THUMB: &str = "█";
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
    format!("{}{} {:.1}s{}", GLYPH_TOOL, name, duration_secs, INDICATOR_COLLAPSED)
}

pub fn turn_complete_line(duration_secs: f64) -> String {
    format!("Turn completed in {:.1}s", duration_secs)
}

pub fn thought_summary_line(first_line: &str) -> String {
    format!("{}{}", first_line, INDICATOR_COLLAPSED)
}
