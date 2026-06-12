//! Theme module - re-exports theme types and builders.

mod crush_grok;
mod grok_day;
mod grok_night;
mod rose_pine_moon;
mod silkcircuit;
mod tokyo_night;

pub use crush_grok::build_crush_grok;
pub use grok_day::build_grok_day;
pub use grok_night::build_grok_night;
pub use rose_pine_moon::build_rose_pine_moon;
pub use silkcircuit::build_silkcircuit;
pub use tokyo_night::build_tokyo_night;

use opaline::Theme;
use ratatui::style::Style;

// ─── Theme Name Resolution ───────────────────────────────────────────────────

const ALL_THEMES: &[&str] = &[
    "crush_grok",
    "silkcircuit_neon",
    "grok_night",
    "grok_day",
    "tokyo_night",
    "rose_pine_moon",
];

const THEME_ALIASES: &[(&str, &str)] = &[
    ("grok_night", "grok_night"),
    ("groknight", "grok_night"),
    ("dark", "grok_night"),
    ("grok_day", "grok_day"),
    ("grokday", "grok_day"),
    ("light", "grok_day"),
    ("day", "grok_day"),
    ("tokyo_night", "tokyo_night"),
    ("tokyonight", "tokyo_night"),
    ("tokyo", "tokyo_night"),
    ("rose_pine_moon", "rose_pine_moon"),
    ("rosepine", "rose_pine_moon"),
    ("rose-pine", "rose_pine_moon"),
];

/// Resolve a theme name or alias to canonical name.
pub fn resolve_theme(name: &str) -> Option<String> {
    let name_lower = name.to_lowercase();
    for (alias, canonical) in THEME_ALIASES {
        if alias.to_lowercase() == name_lower {
            return Some(canonical.to_string());
        }
    }
    for theme in ALL_THEMES {
        if theme.to_lowercase() == name_lower {
            return Some(theme.to_string());
        }
    }
    None
}

// ─── ThemeWrapper ────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct ThemeWrapper {
    inner: Theme,
    name: String,
}

const THEMES: &[(&str, fn() -> ThemeWrapper)] = &[
    ("silkcircuit_neon", ThemeWrapper::silkcircuit_neon),
    ("crush_grok", ThemeWrapper::crush_grok),
    ("grok_night", ThemeWrapper::grok_night),
    ("grok_day", ThemeWrapper::grok_day),
    ("tokyo_night", ThemeWrapper::tokyo_night),
    ("rose_pine_moon", ThemeWrapper::rose_pine_moon),
];

impl ThemeWrapper {
    pub fn color(&self, token: &str) -> opaline::OpalineColor {
        self.inner.color(token)
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    #[cfg(test)]
    pub fn default_for_test() -> Self {
        Self::default()
    }

    pub fn silkcircuit_neon() -> Self {
        Self {
            inner: Theme::from(build_silkcircuit()),
            name: "silkcircuit_neon".to_string(),
        }
    }

    pub fn crush_grok() -> Self {
        Self {
            inner: Theme::from(build_crush_grok()),
            name: "crush_grok".to_string(),
        }
    }

    pub fn grok_night() -> Self {
        Self {
            inner: Theme::from(build_grok_night()),
            name: "grok_night".to_string(),
        }
    }

    pub fn grok_day() -> Self {
        Self {
            inner: Theme::from(build_grok_day()),
            name: "grok_day".to_string(),
        }
    }

    pub fn tokyo_night() -> Self {
        Self {
            inner: Theme::from(build_tokyo_night()),
            name: "tokyo_night".to_string(),
        }
    }

    pub fn rose_pine_moon() -> Self {
        Self {
            inner: Theme::from(build_rose_pine_moon()),
            name: "rose_pine_moon".to_string(),
        }
    }

    pub fn from_name(name: &str) -> Option<Self> {
        let name_lower = name.to_lowercase();
        THEMES.iter()
            .find(|(n, _)| n.to_lowercase() == name_lower)
            .map(|(_, builder)| builder())
    }

    /// Cycles between all available themes.
    pub fn cycle_theme(current: &str) -> Self {
        let idx = ALL_THEMES.iter().position(|&t| t == current);
        let next_idx = idx.map(|i| (i + 1) % ALL_THEMES.len()).unwrap_or(0);
        let next = ALL_THEMES[next_idx];
        Self::from_name(next).unwrap_or_else(Self::crush_grok)
    }

    // ─── Style Builders ────────────────────────────────────────────────────────

    pub fn input_bar_style(&self) -> Style {
        Style::default().fg(self.color("text.primary").into())
    }

    pub fn chevron_style(&self) -> Style {
        Style::default().fg(self.color("accent.user").into())
    }

    pub fn menu_selected_style(&self) -> Style {
        Style::default().fg(self.color("accent.primary").into()).add_modifier(ratatui::style::Modifier::BOLD)
    }

    pub fn menu_unselected_style(&self) -> Style {
        Style::default().fg(self.color("text.secondary").into())
    }

    pub fn divider_style(&self) -> Style {
        Style::default().fg(self.color("border.unfocused").into())
    }

    pub fn tip_style(&self) -> Style {
        Style::default().fg(self.color("text.muted").into())
    }

    pub fn version_style(&self) -> Style {
        Style::default().fg(self.color("text.dim").into())
    }

    pub fn muted_style(&self) -> Style {
        Style::default().fg(self.color("text.muted").into())
    }
}

impl Default for ThemeWrapper {
    fn default() -> Self {
        Self::crush_grok()
    }
}

impl From<Theme> for ThemeWrapper {
    fn from(inner: Theme) -> Self {
        Self { inner, name: "unknown".to_string() }
    }
}

// ─── ThemeColors ─────────────────────────────────────────────────────────────

/// Pre-extracted theme colors for hot-path rendering.
#[derive(Debug, Clone)]
pub struct ThemeColors {
    pub bg_base: ratatui::style::Color,
    pub bg_panel: ratatui::style::Color,
    pub accent_primary: ratatui::style::Color,
    pub accent_secondary: ratatui::style::Color,
    pub text_primary: ratatui::style::Color,
    pub text_secondary: ratatui::style::Color,
    pub text_dim: ratatui::style::Color,
    pub text_muted: ratatui::style::Color,
    pub border_unfocused: ratatui::style::Color,
    pub success: ratatui::style::Color,
    pub error: ratatui::style::Color,
    pub warning: ratatui::style::Color,
    pub syntax_phase: ratatui::style::Color,
    pub text_plan: ratatui::style::Color,
    pub feed_tool_bar: ratatui::style::Color,
    pub accent_user: ratatui::style::Color,
    pub accent_assistant: ratatui::style::Color,
    pub accent_thinking: ratatui::style::Color,
    pub accent_tool: ratatui::style::Color,
    pub accent_system: ratatui::style::Color,
    pub accent_error: ratatui::style::Color,
    pub accent_success: ratatui::style::Color,
    pub accent_running: ratatui::style::Color,
    pub accent_skill: ratatui::style::Color,
    pub accent_plan: ratatui::style::Color,
    pub accent_feedback: ratatui::style::Color,
    pub accent_model: ratatui::style::Color,
    pub accent_teal: ratatui::style::Color,
    pub accent_orange: ratatui::style::Color,
    pub accent_purple: ratatui::style::Color,
    pub accent_yellow: ratatui::style::Color,
    pub accent_blue_bright: ratatui::style::Color,
    pub command: ratatui::style::Color,
    pub path: ratatui::style::Color,
    pub running: ratatui::style::Color,
    pub fuzzy_accent: ratatui::style::Color,
    pub editor_bg: ratatui::style::Color,
    pub surface_bg: ratatui::style::Color,
    pub popover_bg: ratatui::style::Color,
}

impl From<&ThemeWrapper> for ThemeColors {
    fn from(theme: &ThemeWrapper) -> Self {
        Self {
            bg_base: c(theme, "bg.base"), bg_panel: c(theme, "bg.panel"),
            accent_primary: c(theme, "accent.primary"), accent_secondary: c(theme, "accent.secondary"),
            text_primary: c(theme, "text.primary"), text_secondary: c(theme, "text.secondary"),
            text_dim: c(theme, "text.dim"), text_muted: c(theme, "text.muted"),
            border_unfocused: c(theme, "border.unfocused"), success: c(theme, "success"),
            error: c(theme, "error"), warning: c(theme, "warning"),
            syntax_phase: c(theme, "syntax.phase"), text_plan: c(theme, "text.plan"),
            feed_tool_bar: c(theme, "feed.tool.bar"), accent_user: c(theme, "accent.user"),
            accent_assistant: c(theme, "accent.assistant"), accent_thinking: c(theme, "accent.thinking"),
            accent_tool: c(theme, "accent.tool"), accent_system: c(theme, "accent.system"),
            accent_error: c(theme, "accent.error"), accent_success: c(theme, "accent.success"),
            accent_running: c(theme, "accent.running"), accent_skill: c(theme, "accent.skill"),
            accent_plan: c(theme, "accent.plan"), accent_feedback: c(theme, "accent.feedback"),
            accent_model: c(theme, "accent.model"), accent_teal: c(theme, "accent.teal"),
            accent_orange: c(theme, "accent.orange"), accent_purple: c(theme, "accent.purple"),
            accent_yellow: c(theme, "accent.yellow"), accent_blue_bright: c(theme, "accent.blue_bright"),
            command: c(theme, "command"), path: c(theme, "path"), running: c(theme, "running"),
            fuzzy_accent: c(theme, "fuzzy.accent"), editor_bg: c(theme, "editor.bg"),
            surface_bg: c(theme, "surface.bg"), popover_bg: c(theme, "popover.bg"),
        }
    }
}

fn c(theme: &ThemeWrapper, token: &str) -> ratatui::style::Color {
    theme.color(token).into()
}

// Alias for backwards compatibility
pub type ColorPalette = ThemeWrapper;

// Re-export for lib.rs
pub use opaline::OpalineColor;

// ─── Terminal Color Capability Detection ────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ColorCapability {
    Truecolor,
    Color256,
    Color16,
    Monochrome,
}

impl ColorCapability {
    /// Detect terminal color capability.
    pub fn detect() -> Self {
        if let Ok(colorterm) = std::env::var("COLORTERM") {
            return Self::from_colorterm(&colorterm);
        }
        if let Ok(term) = std::env::var("TERM") {
            return Self::from_term(&term);
        }
        ColorCapability::Color256
    }

    fn from_colorterm(colorterm: &str) -> Self {
        if colorterm.contains("truecolor") || colorterm.contains("24bit") {
            ColorCapability::Truecolor
        } else if colorterm.contains("256") {
            ColorCapability::Color256
        } else {
            ColorCapability::Color256
        }
    }

    fn from_term(term: &str) -> Self {
        if term.contains("truecolor") || term.contains("24bit") {
            ColorCapability::Truecolor
        } else if term.contains("256") {
            ColorCapability::Color256
        } else if term.contains("mono") || term.contains("gray") || term.contains("grayscale") {
            ColorCapability::Monochrome
        } else {
            ColorCapability::Color256
        }
    }

    /// Quantize a 24-bit color to the terminal's capability.
    pub fn quantize(self, r: u8, g: u8, b: u8) -> (u8, u8, u8) {
        match self {
            ColorCapability::Truecolor => (r, g, b),
            ColorCapability::Color256 => Self::q256(r, g, b),
            ColorCapability::Color16 => Self::q16(r, g, b),
            ColorCapability::Monochrome => Self::qmono(r, g, b),
        }
    }

    fn q256(r: u8, g: u8, b: u8) -> (u8, u8, u8) {
        let r_idx = ((r as u16 * 5 / 255) as u8).saturating_sub(1).max(0);
        let g_idx = ((g as u16 * 5 / 255) as u8).saturating_sub(1).max(0);
        let b_idx = ((b as u16 * 5 / 255) as u8).saturating_sub(1).max(0);
        (r_idx * 51, g_idx * 51, b_idx * 51)
    }

    fn q16(r: u8, g: u8, b: u8) -> (u8, u8, u8) {
        let gray = (r as u16 + g as u16 + b as u16) / 3;
        let gq = (gray * 15 / 255) as u8;
        let intensity = u8::from(r > 128 || g > 128 || b > 128) * 8;
        let idx = gq + intensity;
        let v = u8::from(idx & 8 != 0) * 127 + 128;
        let v2 = u8::from(idx & 4 != 0) * 127 + 128;
        let v3 = u8::from(idx & 2 != 0) * 127 + 128;
        (v, v2, v3)
    }

    fn qmono(r: u8, g: u8, b: u8) -> (u8, u8, u8) {
        let gray = ((r as u32 * 299 + g as u32 * 587 + b as u32 * 114) / 1000).min(255) as u8;
        (gray, gray, gray)
    }
}

// ─── Extra Tokens (shared across themes) ────────────────────────────────────

fn add_extra_tokens(b: opaline::ThemeBuilder) -> opaline::ThemeBuilder {
    b.token("syntax.phase", opaline::OpalineColor::new(0x6B, 0x50, 0xFF))
        .token("editor.bg", opaline::OpalineColor::new(0x20, 0x20, 0x20))
        .token("surface.bg", opaline::OpalineColor::new(0x20, 0x20, 0x20))
        .token("popover.bg", opaline::OpalineColor::new(0x1A, 0x1A, 0x1A))
        .token("accent.teal", opaline::OpalineColor::new(0x29, 0xC6, 0xBE))
        .token("accent.orange", opaline::OpalineColor::new(0xD5, 0x95, 0x56))
        .token("accent.purple", opaline::OpalineColor::new(0xBC, 0x97, 0xFF))
        .token("accent.yellow", opaline::OpalineColor::new(0xCF, 0xB4, 0x7C))
        .token("accent.blue_bright", opaline::OpalineColor::new(0x88, 0xA6, 0xFF))
}

pub(crate) fn apply_extra_tokens(b: opaline::ThemeBuilder) -> opaline::ThemeBuilder {
    add_extra_tokens(b)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_resolve_theme() {
        assert_eq!(resolve_theme("grok_night"), Some("grok_night".to_string()));
        assert_eq!(resolve_theme("groknight"), Some("grok_night".to_string()));
        assert_eq!(resolve_theme("dark"), Some("grok_night".to_string()));
        assert_eq!(resolve_theme("tokyonight"), Some("tokyo_night".to_string()));
        assert_eq!(resolve_theme("rosepine"), Some("rose_pine_moon".to_string()));
        assert_eq!(resolve_theme("nonexistent"), None);
    }

    #[test]
    fn test_cycle_theme() {
        let t = ThemeWrapper::crush_grok();
        assert_eq!(t.name(), "crush_grok");
        let t2 = ThemeWrapper::cycle_theme("crush_grok");
        assert_eq!(t2.name(), "silkcircuit_neon");
        let t3 = ThemeWrapper::cycle_theme("silkcircuit_neon");
        assert_eq!(t3.name(), "grok_night");
    }

    #[test]
    fn test_from_name() {
        assert!(ThemeWrapper::from_name("grok_night").is_some());
        assert!(ThemeWrapper::from_name("grok_day").is_some());
        assert!(ThemeWrapper::from_name("tokyo_night").is_some());
        assert!(ThemeWrapper::from_name("rose_pine_moon").is_some());
        assert!(ThemeWrapper::from_name("invalid").is_none());
    }
}
