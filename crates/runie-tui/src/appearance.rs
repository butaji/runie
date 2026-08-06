//! Opaline-backed appearance projection for Runie's TUI.
//!
//! Grok's two palettes are kept as app-owned Opaline themes; the remaining
//! named variants reuse Opaline's compatible builtins. Widgets consume
//! semantic tokens instead of owning raw RGB constants.

use opaline::{load_from_str, Theme};
use ratatui::style::{Color, Modifier, Style};
use runie_core::types::ThemeKind;

const GROK_NIGHT: &str = r##"
[meta]
name = "GrokNight"
variant = "dark"
version = "1"

[palette]
bg = "#141414"
fg = "#e1e1e1"
muted = "#6c6c6c"
magenta = "#bb9af7"
blue = "#7aa2f7"
yellow = "#e0af68"
green = "#9ece6a"
red = "#f7768e"
selection = "#1c1c1c"
selection_border = "#3c3c41"

[tokens]
"text.primary" = "fg"
"text.secondary" = "fg"
"text.muted" = "muted"
"accent.primary" = "magenta"
"accent.secondary" = "blue"
success = "green"
error = "red"
warning = "yellow"
"bg.base" = "bg"
"bg.panel" = "selection"
"bg.selection" = "selection"
"border.selection" = "selection_border"

[styles]
keyword = { fg = "accent.primary", bold = true }
muted = { fg = "text.muted" }
"##;

const GROK_DAY: &str = r##"
[meta]
name = "GrokDay"
variant = "light"
version = "1"

[palette]
bg = "#eeeeee"
fg = "#262626"
muted = "#767676"
magenta = "#7d4bc6"
blue = "#2f64d2"
yellow = "#a27612"
green = "#378e23"
red = "#cd3048"
selection = "#e4e4e4"
selection_border = "#b9b9be"

[tokens]
"text.primary" = "fg"
"text.secondary" = "fg"
"text.muted" = "muted"
"accent.primary" = "magenta"
"accent.secondary" = "blue"
success = "green"
error = "red"
warning = "yellow"
"bg.base" = "bg"
"bg.panel" = "selection"
"bg.selection" = "selection"
"border.selection" = "selection_border"

[styles]
keyword = { fg = "accent.primary", bold = true }
muted = { fg = "text.muted" }
"##;

pub fn load(theme: ThemeKind) -> Theme {
    match theme {
        ThemeKind::GrokNight => load_from_str(GROK_NIGHT, None).expect("valid GrokNight theme"),
        ThemeKind::GrokDay => load_from_str(GROK_DAY, None).expect("valid GrokDay theme"),
        ThemeKind::TokyoNight => builtin("tokyo-night"),
        ThemeKind::RosePineMoon => builtin("rose-pine-moon"),
        ThemeKind::OscuraMidnight => builtin("night-owl"),
        ThemeKind::Auto => load(ThemeKind::GrokNight),
    }
}

/// Theme-aware semantic projections for ratatui widgets.
pub fn base_style_for(theme: ThemeKind) -> Style {
    Style::default()
        .fg(token_color(theme, "text.primary"))
        .bg(token_color(theme, "bg.base"))
}

pub fn background_style_for(theme: ThemeKind) -> Style {
    Style::default().bg(token_color(theme, "bg.base"))
}

pub fn user_style_for(theme: ThemeKind) -> Style {
    Style::default()
        .fg(token_color(theme, "text.primary"))
        .bg(token_color(theme, "bg.panel"))
}

pub fn muted_style_for(theme: ThemeKind) -> Style {
    base_style_for(theme).fg(token_color(theme, "text.muted"))
}

pub fn accent_style_for(theme: ThemeKind) -> Style {
    base_style_for(theme).fg(token_color(theme, "accent.primary"))
}

pub fn success_style_for(theme: ThemeKind) -> Style {
    base_style_for(theme).fg(token_color(theme, "success"))
}

pub fn error_style_for(theme: ThemeKind) -> Style {
    base_style_for(theme)
        .fg(token_color(theme, "error"))
        .add_modifier(Modifier::BOLD)
}

pub fn secondary_style_for(theme: ThemeKind) -> Style {
    base_style_for(theme).fg(token_color(theme, "accent.secondary"))
}

pub fn warning_style_for(theme: ThemeKind) -> Style {
    base_style_for(theme).fg(token_color(theme, "warning"))
}

pub fn selected_style_for(theme: ThemeKind) -> Style {
    Style::default().bg(token_color(theme, "bg.selection"))
}

pub fn selected_border_style_for(theme: ThemeKind) -> Style {
    Style::default().fg(token_color(theme, "border.selection"))
}

pub fn base_style() -> Style {
    base_style_for(ThemeKind::GrokNight)
}

pub fn muted_style() -> Style {
    muted_style_for(ThemeKind::GrokNight)
}

pub fn accent_style() -> Style {
    accent_style_for(ThemeKind::GrokNight)
}

pub fn success_style() -> Style {
    success_style_for(ThemeKind::GrokNight)
}

pub fn error_style() -> Style {
    error_style_for(ThemeKind::GrokNight)
}

pub fn secondary_style() -> Style {
    secondary_style_for(ThemeKind::GrokNight)
}

pub fn warning_style() -> Style {
    warning_style_for(ThemeKind::GrokNight)
}

fn token_color(theme: ThemeKind, token: &str) -> Color {
    let color = load(theme).color(token);
    Color::Rgb(color.r, color.g, color.b)
}

fn builtin(name: &str) -> Theme {
    opaline::builtins::load_by_name(name).unwrap_or_else(|| load(ThemeKind::GrokNight))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn grok_palettes_are_real_opaline_themes() {
        assert!(load(ThemeKind::GrokNight).has_token("accent.primary"));
        assert!(load(ThemeKind::GrokDay).is_light());
        assert!(load(ThemeKind::TokyoNight).has_token("code.keyword"));
    }

    #[test]
    fn grok_night_widget_projection_keeps_truecolor_base_and_accent() {
        assert_eq!(base_style().fg, Some(Color::Rgb(225, 225, 225)));
        assert_eq!(base_style().bg, Some(Color::Rgb(20, 20, 20)));
        assert_eq!(accent_style().fg, Some(Color::Rgb(187, 154, 247)));
    }

    #[test]
    fn themed_projection_resolves_day_tokens_without_night_literals() {
        assert_eq!(
            base_style_for(ThemeKind::GrokDay).fg,
            Some(Color::Rgb(38, 38, 38))
        );
        assert_eq!(
            base_style_for(ThemeKind::GrokDay).bg,
            Some(Color::Rgb(238, 238, 238))
        );
        assert_eq!(
            accent_style_for(ThemeKind::GrokDay).fg,
            Some(Color::Rgb(125, 75, 198))
        );
        assert_eq!(
            selected_style_for(ThemeKind::GrokNight).bg,
            Some(Color::Rgb(28, 28, 28))
        );
        assert_eq!(
            selected_style_for(ThemeKind::GrokDay).bg,
            Some(Color::Rgb(228, 228, 228))
        );
    }
}
