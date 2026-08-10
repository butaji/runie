//! Opaline-backed appearance projection for Runie's TUI.
//!
//! Grok's two palettes are kept as app-owned Opaline themes; the remaining
//! named variants reuse Opaline's compatible builtins. Widgets consume
//! semantic tokens instead of owning raw RGB constants.

use crate::view::PaintIntent;
use opaline::{load_from_str, Theme};
use ratatui::style::{Color, Modifier, Style};
use runie_core::types::ThemeKind;
use runie_tui_model::ThemeToken;

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
thought_accent = "#685786"
yellow = "#e0af68"
green = "#9ece6a"
red = "#f7768e"
selection = "#1c1c1c"
panel = "#242424"
prompt_border = "#505058"
footer_key = "#c8c8c8"
assistant_body = "#c8c8c8"
header_path = "#585858"
header_meter = "#e0e0e0"
model_caption = "#808080"
diff_delete = "#420e14"
diff_insert = "#063806"
selection_border = "#3c3c41"

[tokens]
"text.primary" = "fg"
"text.secondary" = "fg"
"text.muted" = "muted"
"accent.primary" = "magenta"
"accent.secondary" = "blue"
"accent.thought" = "thought_accent"
success = "green"
error = "red"
warning = "yellow"
"bg.base" = "bg"
"bg.panel" = "panel"
"border.prompt" = "prompt_border"
"text.footer_key" = "footer_key"
"text.assistant" = "assistant_body"
"text.header_path" = "header_path"
"text.header_meter" = "header_meter"
"text.model" = "model_caption"
"bg.diff_delete" = "diff_delete"
"bg.diff_insert" = "diff_insert"
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
thought_accent = "#b59cda"
yellow = "#a27612"
green = "#378e23"
red = "#cd3048"
selection = "#e4e4e4"
panel = "#dedede"
prompt_border = "#b9b9be"
footer_key = "#262626"
assistant_body = "#262626"
header_path = "#767676"
header_meter = "#262626"
model_caption = "#606060"
diff_delete = "#f5dade"
diff_insert = "#daf2dc"
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
"bg.panel" = "panel"
"border.prompt" = "prompt_border"
"text.footer_key" = "footer_key"
"text.assistant" = "assistant_body"
"text.header_path" = "header_path"
"text.header_meter" = "header_meter"
"text.model" = "model_caption"
"bg.diff_delete" = "diff_delete"
"bg.diff_insert" = "diff_insert"
"bg.selection" = "selection"
"border.selection" = "selection_border"

[styles]
keyword = { fg = "accent.primary", bold = true }
muted = { fg = "text.muted" }
"##;

macro_rules! builtin_themes {
    ($($variant:ident => $name:literal),+ $(,)?) => {
        fn load_builtin(theme: ThemeKind) -> Theme {
            match theme {
                $(ThemeKind::$variant => builtin($name),)+
                _ => unreachable!("special themes handled by load"),
            }
        }

        #[cfg(test)]
        const BUILTIN_THEMES: &[(ThemeKind, &str)] = &[
            $((ThemeKind::$variant, $name),)+
        ];
    };
}

builtin_themes! {
    TokyoNight => "tokyo-night",
    RosePineMoon => "rose-pine-moon",
    OscuraMidnight => "night-owl",
    AyuDark => "ayu-dark",
    AyuLight => "ayu-light",
    AyuMirage => "ayu-mirage",
    CatppuccinFrappe => "catppuccin-frappe",
    CatppuccinLatte => "catppuccin-latte",
    CatppuccinMacchiato => "catppuccin-macchiato",
    CatppuccinMocha => "catppuccin-mocha",
    Dracula => "dracula",
    EverforestDark => "everforest-dark",
    EverforestLight => "everforest-light",
    FlexokiDark => "flexoki-dark",
    FlexokiLight => "flexoki-light",
    GithubDarkDimmed => "github-dark-dimmed",
    GithubLight => "github-light",
    GruvboxDark => "gruvbox-dark",
    GruvboxLight => "gruvbox-light",
    KanagawaDragon => "kanagawa-dragon",
    KanagawaLotus => "kanagawa-lotus",
    KanagawaWave => "kanagawa-wave",
    LightOwl => "light-owl",
    MonokaiPro => "monokai-pro",
    Nord => "nord",
    OneDark => "one-dark",
    OneLight => "one-light",
    Palenight => "palenight",
    RosePine => "rose-pine",
    RosePineDawn => "rose-pine-dawn",
    SilkCircuitDawn => "silkcircuit-dawn",
    SilkCircuitGlow => "silkcircuit-glow",
    SilkCircuitNeon => "silkcircuit-neon",
    SilkCircuitSoft => "silkcircuit-soft",
    SilkCircuitVibrant => "silkcircuit-vibrant",
    SolarizedDark => "solarized-dark",
    SolarizedLight => "solarized-light",
    TokyoNightMoon => "tokyo-night-moon",
    TokyoNightStorm => "tokyo-night-storm",
}

pub fn load(theme: ThemeKind) -> Theme {
    match theme {
        ThemeKind::GrokNight => load_from_str(GROK_NIGHT, None).expect("valid GrokNight theme"),
        ThemeKind::GrokDay => load_from_str(GROK_DAY, None).expect("valid GrokDay theme"),
        ThemeKind::Auto => load(ThemeKind::GrokNight),
        ThemeKind::TerminalNative => load(ThemeKind::GrokNight),
        _ => load_builtin(theme),
    }
}

/// Theme-aware semantic projections for ratatui widgets.
pub fn base_style_for(theme: ThemeKind) -> Style {
    style_for_intent(theme, PaintIntent::Base)
}

/// Resolve a declarative paint intent at the terminal boundary.
pub fn style_for_intent(theme: ThemeKind, intent: PaintIntent) -> Style {
    match intent {
        PaintIntent::Base => Style::default()
            .fg(token_color(theme, ThemeToken::TextPrimary))
            .bg(token_color(theme, ThemeToken::BackgroundBase)),
        PaintIntent::Panel => Style::default()
            .fg(token_color(theme, ThemeToken::TextPrimary))
            .bg(token_color(theme, ThemeToken::BackgroundPanel)),
        PaintIntent::Muted => muted_style_for(theme),
        PaintIntent::Accent => accent_style_for(theme),
        PaintIntent::SecondaryAccent => secondary_style_for(theme),
        PaintIntent::Success => success_style_for(theme),
        PaintIntent::Error => error_style_for(theme),
        PaintIntent::Warning => warning_style_for(theme),
        PaintIntent::Selection => selected_style_for(theme),
        PaintIntent::SelectionBorder => selected_border_style_for(theme),
        PaintIntent::DiffInsert => diff_insert_style_for(theme),
        PaintIntent::DiffDelete => diff_delete_style_for(theme),
        PaintIntent::FooterKey => footer_key_style_for(theme),
    }
}

pub fn background_style_for(theme: ThemeKind) -> Style {
    Style::default().bg(token_color(theme, ThemeToken::BackgroundBase))
}

pub fn user_style_for(theme: ThemeKind) -> Style {
    style_for_intent(theme, PaintIntent::Panel)
}

pub fn panel_background_style_for(theme: ThemeKind) -> Style {
    Style::default().bg(token_color(theme, ThemeToken::BackgroundPanel))
}

pub fn diff_insert_style_for(theme: ThemeKind) -> Style {
    success_style_for(theme).bg(token_color(theme, ThemeToken::BackgroundDiffInsert))
}

pub fn diff_delete_style_for(theme: ThemeKind) -> Style {
    error_style_for(theme).bg(token_color(theme, ThemeToken::BackgroundDiffDelete))
}

pub fn muted_style_for(theme: ThemeKind) -> Style {
    base_style_for(theme).fg(token_color(theme, ThemeToken::TextMuted))
}

pub fn accent_style_for(theme: ThemeKind) -> Style {
    base_style_for(theme).fg(token_color(theme, ThemeToken::AccentPrimary))
}

pub fn thought_accent_style_for(theme: ThemeKind) -> Style {
    base_style_for(theme).fg(token_color(theme, ThemeToken::AccentThought))
}

pub fn success_style_for(theme: ThemeKind) -> Style {
    base_style_for(theme).fg(token_color(theme, ThemeToken::Success))
}

pub fn error_style_for(theme: ThemeKind) -> Style {
    base_style_for(theme)
        .fg(token_color(theme, ThemeToken::Error))
        .add_modifier(Modifier::BOLD)
}

pub fn secondary_style_for(theme: ThemeKind) -> Style {
    base_style_for(theme).fg(token_color(theme, ThemeToken::AccentSecondary))
}

pub fn warning_style_for(theme: ThemeKind) -> Style {
    base_style_for(theme).fg(token_color(theme, ThemeToken::Warning))
}

pub fn selected_style_for(theme: ThemeKind) -> Style {
    Style::default().bg(token_color(theme, ThemeToken::BackgroundSelection))
}

pub fn selected_border_style_for(theme: ThemeKind) -> Style {
    Style::default().fg(token_color(theme, ThemeToken::BorderSelection))
}

pub fn prompt_border_style_for(theme: ThemeKind) -> Style {
    base_style_for(theme).fg(token_color(theme, ThemeToken::BorderPrompt))
}

pub fn footer_key_style_for(theme: ThemeKind) -> Style {
    base_style_for(theme).fg(token_color(theme, ThemeToken::TextFooterKey))
}

pub fn assistant_body_style_for(theme: ThemeKind) -> Style {
    base_style_for(theme).fg(token_color(theme, ThemeToken::TextAssistant))
}

pub fn header_path_style_for(theme: ThemeKind) -> Style {
    base_style_for(theme).fg(token_color(theme, ThemeToken::TextHeaderPath))
}

pub fn model_caption_style_for(theme: ThemeKind) -> Style {
    base_style_for(theme).fg(token_color(theme, ThemeToken::TextModel))
}

pub fn header_meter_style_for(theme: ThemeKind) -> Style {
    base_style_for(theme).fg(token_color(theme, ThemeToken::TextHeaderMeter))
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

fn token_color(theme: ThemeKind, token: ThemeToken) -> Color {
    if theme == ThemeKind::TerminalNative {
        return Color::Reset;
    }
    let color = load(theme).color(token.opaline_name());
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
    fn every_declared_builtin_theme_resolves_from_the_schema() {
        for &(theme, _) in BUILTIN_THEMES {
            assert!(load(theme).has_token("code.keyword"), "{theme:?}");
        }
    }

    #[test]
    fn grok_night_widget_projection_keeps_truecolor_base_and_accent() {
        assert_eq!(base_style().fg, Some(Color::Rgb(225, 225, 225)));
        assert_eq!(base_style().bg, Some(Color::Rgb(20, 20, 20)));
        assert_eq!(accent_style().fg, Some(Color::Rgb(187, 154, 247)));
    }

    #[test]
    fn grok_night_feed_semantic_tokens_match_grok_palette() {
        let base = base_style_for(ThemeKind::GrokNight);
        let user = user_style_for(ThemeKind::GrokNight);
        let muted = muted_style_for(ThemeKind::GrokNight);
        let selection = selected_style_for(ThemeKind::GrokNight);

        assert_eq!(base.fg, Some(Color::Rgb(225, 225, 225)));
        assert_eq!(base.bg, Some(Color::Rgb(20, 20, 20)));
        assert_eq!(user.bg, Some(Color::Rgb(36, 36, 36)));
        assert_eq!(muted.fg, Some(Color::Rgb(108, 108, 108)));
        assert_eq!(selection.bg, Some(Color::Rgb(28, 28, 28)));
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
        assert_eq!(
            user_style_for(ThemeKind::GrokNight).bg,
            Some(Color::Rgb(36, 36, 36))
        );
        assert_eq!(
            user_style_for(ThemeKind::GrokDay).bg,
            Some(Color::Rgb(222, 222, 222))
        );
    }

    #[test]
    fn terminal_native_theme_projects_default_terminal_colors() {
        let native_base = Style::default().fg(Color::Reset).bg(Color::Reset);
        assert_eq!(base_style_for(ThemeKind::TerminalNative), native_base);
        assert_eq!(
            background_style_for(ThemeKind::TerminalNative),
            Style::default().bg(Color::Reset)
        );
        assert_eq!(accent_style_for(ThemeKind::TerminalNative), native_base);
    }

    #[test]
    fn every_declarative_paint_intent_resolves_for_both_grok_themes() {
        let intents = [
            PaintIntent::Base,
            PaintIntent::Panel,
            PaintIntent::Muted,
            PaintIntent::Accent,
            PaintIntent::SecondaryAccent,
            PaintIntent::Success,
            PaintIntent::Error,
            PaintIntent::Warning,
            PaintIntent::Selection,
            PaintIntent::SelectionBorder,
            PaintIntent::DiffInsert,
            PaintIntent::DiffDelete,
        ];
        for theme in [ThemeKind::GrokNight, ThemeKind::GrokDay] {
            for intent in intents {
                assert_ne!(style_for_intent(theme, intent), Style::default());
            }
        }
    }
}
