//! Theme system powered by opaline
//!
//! Runie-specific styles are registered as defaults so any theme can override them.
//! The current theme is cached in a global lock; `draw_snapshot` sets it at frame start.

pub use crate::semantic_tokens::SemanticTokens;

use ratatui::layout::Alignment;
use ratatui::style::{Color, Style};
use ratatui::text::Line;
use ratatui::widgets::{Block, BorderType, Borders, TitlePosition};
use std::sync::{Arc, Mutex, RwLock};

static CURRENT_THEME: RwLock<Option<Arc<opaline::Theme>>> = RwLock::new(None);
static CURRENT_THEME_NAME: Mutex<String> = Mutex::new(String::new());
static CURRENT_CAPS: RwLock<Option<crate::terminal::caps::TerminalCapabilities>> =
    RwLock::new(None);

#[cfg(test)]
pub(crate) static TEST_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

#[cfg(test)]
pub fn test_lock() -> std::sync::MutexGuard<'static, ()> {
    TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner())
}

/// Set the active theme by name. Called by `draw_snapshot` at frame start.
/// This is a no-op when the requested theme is already active.
pub fn set_current_theme(name: &str) {
    set_current_theme_with_caps(name, crate::terminal::caps::TerminalCapabilities::default());
}

/// Set the active theme by name, quantized to the given terminal capabilities.
/// Quantization happens once at load time; per-frame rendering is unaffected.
pub fn set_current_theme_with_caps(
    name: &str,
    caps: crate::terminal::caps::TerminalCapabilities,
) {
    {
        let mut current = CURRENT_CAPS.write().unwrap_or_else(|e| e.into_inner());
        *current = Some(caps);
    }
    {
        let mut current = CURRENT_THEME_NAME.lock().unwrap_or_else(|e| e.into_inner());
        if current.as_str() == name {
            return;
        }
        *current = name.to_string();
    }
    let theme = load_theme_with_caps(name, caps);
    let mut guard = CURRENT_THEME.write().unwrap_or_else(|e| e.into_inner());
    *guard = Some(Arc::new(theme));
}

pub const DEFAULT_THEME_NAME: &str = "runie";

/// Get the name of the currently active theme.
pub fn current_theme_name() -> String {
    CURRENT_THEME_NAME
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .clone()
}

/// Get the currently active theme (falls back to default).
pub fn current_theme() -> Arc<opaline::Theme> {
    let guard = CURRENT_THEME.read().unwrap_or_else(|e| e.into_inner());
    guard.clone().unwrap_or_else(|| Arc::new(default_theme()))
}

/// Load a theme by name: builtin → custom file → default fallback (no style registration).
fn load_theme_raw(name: &str) -> opaline::Theme {
    // Only use the builtin loader if the name is actually a builtin.
    // "runie" is not a builtin — it uses the embedded DEFAULT_THEME_TOML.
    if let Some(t) = opaline::load_by_name(name) {
        return t;
    }
    let custom_path = dirs::home_dir()
        .unwrap_or_else(|| std::path::PathBuf::from("."))
        .join(".runie")
        .join("themes")
        .join(format!("{}.toml", name));
    if let Ok(theme) = opaline::load_from_file(&custom_path) {
        return theme;
    }
    default_theme()
}

/// Load a theme by name: builtin → custom file → default fallback.
fn load_theme(name: &str) -> opaline::Theme {
    register_runie_styles(load_theme_raw(name))
}

/// Load a theme and quantize its colors to the terminal's color depth.
fn load_theme_with_caps(
    name: &str,
    caps: crate::terminal::caps::TerminalCapabilities,
) -> opaline::Theme {
    let base = load_theme(name);
    if caps.truecolor {
        return base; // No quantization needed
    }
    quantize_theme(base, caps)
}

/// Quantize all palette and token colors in a theme to the terminal's color depth.
fn quantize_theme(theme: opaline::Theme, caps: crate::terminal::caps::TerminalCapabilities) -> opaline::Theme {
    use opaline::OpalineColor;

    // Determine target depth: ANSI16 if mouse is None (very limited terminal),
    // otherwise ANSI256.
    let depth = if caps.mouse == crate::terminal::caps::MouseCapability::None {
        crate::quantize::ColorDepth::ANSI16
    } else {
        crate::quantize::ColorDepth::ANSI256
    };

    // Collect quantized (name, OpalineColor) pairs from palette and tokens.
    let mut quantized: Vec<(String, OpalineColor)> = Vec::new();

    for name in theme.palette_names() {
        let c = theme.color(name);
        quantized.push((name.to_string(), quantize_opaline_color(c, depth)));
    }
    for name in theme.token_names() {
        let c = theme.color(name);
        quantized.push((name.to_string(), quantize_opaline_color(c, depth)));
    }

    // Reconstruct: load fresh theme and register quantized tokens on top.
    let name = current_theme_name();
    let mut result = load_theme_raw(&name);
    for (k, v) in &quantized {
        result.register_token(k, *v);
    }
    register_runie_styles(result)
}

/// Quantize an opaline color to the given depth, returning the nearest ANSI color.
fn quantize_opaline_color(
    c: opaline::OpalineColor,
    depth: crate::quantize::ColorDepth,
) -> opaline::OpalineColor {
    let rat = Color::Rgb(c.r, c.g, c.b);
    let quantized = crate::quantize::quantize(rat, depth);
    match quantized {
        Color::Indexed(i) => {
            // Map indexed color back to a reasonable RGB approximation.
            indexed_to_opaline(i)
        }
        Color::Rgb(r, g, b) => opaline::OpalineColor::new(r, g, b),
        // Named/other colors pass through as fallback.
        _ => c,
    }
}

/// Approximate an ANSI color index as an OpalineColor (for quantized theme tokens).
fn indexed_to_opaline(i: u8) -> opaline::OpalineColor {
    // ANSI 16-color palette approximations.
    const ANSI16: [(u8, u8, u8); 16] = [
        (0x00, 0x00, 0x00), // 0  black
        (0xCD, 0x00, 0x00), // 1  red
        (0x00, 0xCD, 0x00), // 2  green
        (0xCD, 0xCD, 0x00), // 3  yellow
        (0x00, 0x00, 0xEE), // 4  blue
        (0xCD, 0x00, 0xCD), // 5  magenta
        (0x00, 0xCD, 0xCD), // 6  cyan
        (0xE5, 0xE5, 0xE5), // 7  white
        (0x7F, 0x7F, 0x7F), // 8  bright black
        (0xFF, 0x00, 0x00), // 9  bright red
        (0x00, 0xFF, 0x00), // 10 bright green
        (0xFF, 0xFF, 0x00), // 11 bright yellow
        (0x00, 0x00, 0xFF), // 12 bright blue
        (0xFF, 0x00, 0xFF), // 13 bright magenta
        (0x00, 0xFF, 0xFF), // 14 bright cyan
        (0xFF, 0xFF, 0xFF), // 15 bright white
    ];
    if (i as usize) < ANSI16.len() {
        let (r, g, b) = ANSI16[i as usize];
        opaline::OpalineColor::new(r, g, b)
    } else {
        opaline::OpalineColor::FALLBACK
    }
}

/// List all available builtin theme names.
pub fn list_builtin_themes() -> Vec<&'static str> {
    runie_core::themes::BUILTIN_THEMES.to_vec()
}

fn default_theme() -> opaline::Theme {
    opaline::load_from_str(crate::semantic_tokens::DEFAULT_THEME_TOML, None)
        .expect("embedded default theme must be valid")
}

/// Get semantic tokens from the current theme.
pub fn semantic_tokens() -> SemanticTokens {
    SemanticTokens::from_theme(&current_theme())
}

fn register_runie_styles(mut theme: opaline::Theme) -> opaline::Theme {
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
    theme.register_default_style(
        "runie.diff.file_header",
        opaline::OpalineStyle::fg(dim),
    );
    // Context: plain foreground.
    theme.register_default_style(
        "runie.diff.context",
        opaline::OpalineStyle::fg(theme.color("text.primary")),
    );
}

/// Blend two opaline colors at the given opacity (0.0–1.0).
fn blend_opaline(bg: opaline::OpalineColor, fg: opaline::OpalineColor, opacity: f32) -> opaline::OpalineColor {
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

/// Diff gutter insert background: subtle green tint over base bg.
pub fn color_diff_insert_bg() -> Color {
    let bg = color_bg();
    let success = color_success();
    blend(bg, success, 0.12)
}

/// Diff gutter remove background: subtle red tint over base bg.
pub fn color_diff_remove_bg() -> Color {
    let bg = color_bg();
    let error = color_error();
    blend(bg, error, 0.12)
}

/// Darken an RGB color by a factor (0.0–1.0).
/// Uses palette::Srgb for correct gamma-space darkening.
pub fn darken(color: Color, factor: f32) -> Color {
    match color {
        Color::Rgb(r, g, b) => {
            use palette::Srgb;
            // palette uses 0.0-1.0 range
            let s = Srgb::new(r as f32 / 255.0, g as f32 / 255.0, b as f32 / 255.0);
            // Darken by scaling toward black in display gamma space.
            let factor = factor.clamp(0.0, 1.0);
            Color::Rgb(
                (s.red * factor * 255.0) as u8,
                (s.green * factor * 255.0) as u8,
                (s.blue * factor * 255.0) as u8,
            )
        }
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

/// Blend two RGB colors with the given opacity (0.0-1.0).
/// Uses palette::Srgba for proper premultiplied-alpha blending.
fn blend(bg: Color, fg: Color, opacity: f32) -> Color {
    use palette::Srgba;
    use palette::blend::BlendWith;
    use palette::blend::PreAlpha;
    use palette::IntoColor;
    let opacity = opacity.clamp(0.0, 1.0);

    let (br, bb, bblue) = match bg {
        Color::Rgb(r, g, b) => (r as f32, g as f32, b as f32),
        _ => (30.0, 30.0, 30.0),
    };
    let (fr, fg_g, fb) = match fg {
        Color::Rgb(r, g, b) => (r as f32, g as f32, b as f32),
        _ => return fg,
    };

    // Convert to palette's 0.0-1.0 Srgba space.
    let bg_s: Srgba<f32> = Srgba::new(br / 255.0, bb / 255.0, bblue / 255.0, 1.0);
    let fg_s: Srgba<f32> = Srgba::new(fr / 255.0, fg_g / 255.0, fb / 255.0, opacity);

    // Standard over-compositing with premultiplied alpha.
    let bg_pre: PreAlpha<_> = bg_s.into();
    let fg_pre: PreAlpha<_> = fg_s.into();

    let out: PreAlpha<_> = fg_pre.blend_with(bg_pre, |src: PreAlpha<_>, dst: PreAlpha<_>| {
        // Standard over: dst * (1 - src_alpha) + src
        PreAlpha {
            color: src.color + dst.color * (1.0 - src.alpha),
            alpha: src.alpha + dst.alpha * (1.0 - src.alpha),
        }
    });

    // Convert back to Srgba, then to sRGB.
    let out: Srgba<f32> = out.into();
    Color::Rgb(
        (out.red.clamp(0.0, 1.0) * 255.0) as u8,
        (out.green.clamp(0.0, 1.0) * 255.0) as u8,
        (out.blue.clamp(0.0, 1.0) * 255.0) as u8,
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
style_fn!(style_diff_insert, "runie.diff.insert");
style_fn!(style_diff_remove, "runie.diff.remove");
style_fn!(style_diff_hunk, "runie.diff.hunk");
style_fn!(style_diff_file_header, "runie.diff.file_header");
style_fn!(style_diff_context, "runie.diff.context");

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

// Re-export from core so the layout helper and renderer always agree on
// prefix widths.
pub use runie_core::layout::{GLYPH_AGENT, GLYPH_INDENT, GLYPH_USER};

pub const GLYPH_TOOL: &str = "✓ ";
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn palette_darken_uses_palette_types() {
        // Verify darken works with palette's Srgb.
        let c = Color::Rgb(200, 150, 100);
        let darkened = darken(c, 0.5);
        assert!(matches!(darkened, Color::Rgb(_, _, _)));
    }

    #[test]
    fn palette_blend_uses_palette_types() {
        // Verify blend works with palette's Srgba and PreAlpha.
        let bg = Color::Rgb(30, 30, 30);
        let fg = Color::Rgb(200, 50, 50);
        let result = blend(bg, fg, 0.3);
        assert!(matches!(result, Color::Rgb(r, g, b) if r > 30 && r < 200));
    }

    #[test]
    fn palette_blend_with_zero_opacity_returns_bg() {
        let bg = Color::Rgb(10, 20, 30);
        let fg = Color::Rgb(200, 100, 50);
        let result = blend(bg, fg, 0.0);
        assert!(matches!(result, Color::Rgb(r, g, b) if r == 10 && g == 20 && b == 30));
    }
}
