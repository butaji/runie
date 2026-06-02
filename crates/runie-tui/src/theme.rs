use opaline::{Theme, ThemeVariant};

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
            inner: Theme::from(opaline::builtins::silkcircuit_neon()),
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
        match name {
            "silkcircuit_neon" => Some(Self::silkcircuit_neon()),
            "crush_grok" => Some(Self::crush_grok()),
            "grok_night" => Some(Self::grok_night()),
            "grok_day" => Some(Self::grok_day()),
            "tokyo_night" => Some(Self::tokyo_night()),
            "rose_pine_moon" => Some(Self::rose_pine_moon()),
            _ => None,
        }
    }

    /// Cycles between all available themes.
    pub fn cycle_theme(current: &str) -> Self {
        let idx = ALL_THEMES.iter().position(|&t| t == current);
        let next_idx = idx.map(|i| (i + 1) % ALL_THEMES.len()).unwrap_or(0);
        let next = ALL_THEMES[next_idx];
        Self::from_name(next).unwrap_or_else(Self::crush_grok)
    }
}

fn build_crush_grok() -> Theme {
    let mut b = Theme::builder("crush-grok")
        .variant(ThemeVariant::Dark)
        .author("Runie")
        .description("Crush + GrokBuild hybrid");
    b = add_crush_grok_palettes(b);
    b = add_crush_grok_tokens(b);
    b = add_crush_grok_feed_tokens(b);
    b = add_crush_grok_code_tokens(b);
    b = add_crush_grok_diff_tokens(b);
    add_extra_tokens(b).build()
}

fn build_grok_night() -> Theme {
    let mut b = Theme::builder("grok-night")
        .variant(ThemeVariant::Dark)
        .author("Runie")
        .description("GrokNight - deep navy default dark theme");
    b = add_grok_night_palettes(b);
    b = add_grok_night_tokens(b);
    add_extra_tokens(b).build()
}

fn build_grok_day() -> Theme {
    let mut b = Theme::builder("grok-day")
        .variant(ThemeVariant::Light)
        .author("Runie")
        .description("GrokDay - bright light theme");
    b = add_grok_day_palettes(b);
    b = add_grok_day_tokens(b);
    add_extra_tokens(b).build()
}

fn build_tokyo_night() -> Theme {
    let mut b = Theme::builder("tokyo-night")
        .variant(ThemeVariant::Dark)
        .author("Runie")
        .description("TokyoNight - deep blue-black theme");
    b = add_tokyo_night_palettes(b);
    b = add_tokyo_night_tokens(b);
    add_extra_tokens(b).build()
}

fn build_rose_pine_moon() -> Theme {
    let mut b = Theme::builder("rose-pine-moon")
        .variant(ThemeVariant::Dark)
        .author("Runie")
        .description("RosePineMoon - deep purple-gray theme");
    b = add_rose_pine_moon_palettes(b);
    b = add_rose_pine_moon_tokens(b);
    add_extra_tokens(b).build()
}

// ─── Crush Grok Theme ─────────────────────────────────────────────────────────

fn add_crush_grok_palettes(b: opaline::ThemeBuilder) -> opaline::ThemeBuilder {
    b.palette("pepper", opaline::OpalineColor::new(0x20, 0x1F, 0x26))
        .palette("cosmic", opaline::OpalineColor::new(0x0F, 0x0C, 0x14))
        .palette("charple", opaline::OpalineColor::new(0x6B, 0x50, 0xFF))
        .palette("grok_orange", opaline::OpalineColor::new(0xFF, 0x6B, 0x00))
        .palette("neon_teal", opaline::OpalineColor::new(0x00, 0xF5, 0xD4))
        .palette("dolly_pink", opaline::OpalineColor::new(0xFF, 0x60, 0xFF))
        .palette("butter", opaline::OpalineColor::new(0xFF, 0xFA, 0xF1))
        .palette("ash", opaline::OpalineColor::new(0xDF, 0xDB, 0xDD))
        .palette("smoke", opaline::OpalineColor::new(0xBF, 0xBC, 0xC8))
        .palette("sriracha", opaline::OpalineColor::new(0xEB, 0x42, 0x68))
        .palette("charcoal", opaline::OpalineColor::new(0x3A, 0x39, 0x43))
}

fn add_crush_grok_tokens(b: opaline::ThemeBuilder) -> opaline::ThemeBuilder {
    b.token("bg.base", opaline::OpalineColor::new(0x0F, 0x0C, 0x14))
        .token("bg.light", opaline::OpalineColor::new(0x20, 0x1F, 0x26))
        .token("bg.dark", opaline::OpalineColor::new(0x0A, 0x09, 0x10))
        .token("bg.highlight", opaline::OpalineColor::new(0x2A, 0x29, 0x32))
        .token("bg.hover", opaline::OpalineColor::new(0x35, 0x34, 0x3D))
        .token("bg.terminal", opaline::OpalineColor::new(0x0A, 0x09, 0x10))
        .token("bg.panel", opaline::OpalineColor::new(0x20, 0x1F, 0x26))
        .token("text.primary", opaline::OpalineColor::new(0xFF, 0xFA, 0xF1))
        .token("text.secondary", opaline::OpalineColor::new(0xDF, 0xDB, 0xDD))
        .token("text.muted", opaline::OpalineColor::new(0xBF, 0xBC, 0xC8))
        .token("text.dim", opaline::OpalineColor::new(0x8A, 0x87, 0x94))
        .token("accent.primary", opaline::OpalineColor::new(0xFF, 0x6B, 0x00))
        .token("accent.secondary", opaline::OpalineColor::new(0x00, 0xF5, 0xD4))
        .token("accent.tertiary", opaline::OpalineColor::new(0xFF, 0x60, 0xFF))
        .token("accent.user", opaline::OpalineColor::new(0xFF, 0x6B, 0x00))
        .token("accent.assistant", opaline::OpalineColor::new(0x00, 0xF5, 0xD4))
        .token("accent.thinking", opaline::OpalineColor::new(0xD4, 0xA0, 0x4E))
        .token("accent.tool", opaline::OpalineColor::new(0x6B, 0x50, 0xFF))
        .token("accent.system", opaline::OpalineColor::new(0x3A, 0x39, 0x43))
        .token("accent.error", opaline::OpalineColor::new(0xEB, 0x42, 0x68))
        .token("accent.success", opaline::OpalineColor::new(0x00, 0xF5, 0xD4))
        .token("accent.running", opaline::OpalineColor::new(0xFF, 0x6B, 0x00))
        .token("accent.skill", opaline::OpalineColor::new(0x6B, 0x50, 0xFF))
        .token("accent.plan", opaline::OpalineColor::new(0xD4, 0xA0, 0x4E))
        .token("accent.feedback", opaline::OpalineColor::new(0xFF, 0x60, 0xFF))
        .token("accent.model", opaline::OpalineColor::new(0x00, 0xF5, 0xD4))
        .token("success", opaline::OpalineColor::new(0x00, 0xF5, 0xD4))
        .token("warning", opaline::OpalineColor::new(0xFF, 0x6B, 0x00))
        .token("error", opaline::OpalineColor::new(0xEB, 0x42, 0x68))
        .token("command", opaline::OpalineColor::new(0x6B, 0x50, 0xFF))
        .token("path", opaline::OpalineColor::new(0x00, 0xF5, 0xD4))
        .token("running", opaline::OpalineColor::new(0xFF, 0x6B, 0x00))
        .token("fuzzy.accent", opaline::OpalineColor::new(0xFF, 0x6B, 0x00))
        .token("border.unfocused", opaline::OpalineColor::new(0x3A, 0x39, 0x43))
        .token("border.focused", opaline::OpalineColor::new(0xFF, 0x6B, 0x00))
}

fn add_crush_grok_feed_tokens(b: opaline::ThemeBuilder) -> opaline::ThemeBuilder {
    b.token("feed.user.bar", opaline::OpalineColor::new(0xFF, 0x6B, 0x00))
        .token("feed.assistant.bar", opaline::OpalineColor::new(0x00, 0xF5, 0xD4))
        .token("feed.tool.bar", opaline::OpalineColor::new(0x6B, 0x50, 0xFF))
        .token("feed.agent.bar", opaline::OpalineColor::new(0xFF, 0x60, 0xFF))
        .token("feed.system.bar", opaline::OpalineColor::new(0x3A, 0x39, 0x43))
        .token("feed.user.bg", opaline::OpalineColor::new(0x1A, 0x19, 0x20))
        .token("feed.separator", opaline::OpalineColor::new(0xBF, 0xBC, 0xC8))
}

fn add_crush_grok_code_tokens(b: opaline::ThemeBuilder) -> opaline::ThemeBuilder {
    b.token("code.path", opaline::OpalineColor::new(0x6B, 0x50, 0xFF))
        .token("code.keyword", opaline::OpalineColor::new(0x00, 0xF5, 0xD4))
        .token("code.string", opaline::OpalineColor::new(0xFF, 0x60, 0xFF))
        .token("code.comment", opaline::OpalineColor::new(0x8A, 0x87, 0x94))
        .token("code.type", opaline::OpalineColor::new(0x39, 0xFF, 0x8C))
}

fn add_crush_grok_diff_tokens(b: opaline::ThemeBuilder) -> opaline::ThemeBuilder {
    b.token("diff.removed", opaline::OpalineColor::new(0xFF, 0x6B, 0x6B))
        .token("diff.added", opaline::OpalineColor::new(0x51, 0xCF, 0x66))
        .token("diff.removed_bg", opaline::OpalineColor::new(0x3A, 0x1A, 0x1A))
        .token("diff.added_bg", opaline::OpalineColor::new(0x1A, 0x3A, 0x1A))
        .token("text.plan", opaline::OpalineColor::new(0xD4, 0xA0, 0x4E))
}

// ─── GrokNight Theme ─────────────────────────────────────────────────────────

fn add_grok_night_palettes(b: opaline::ThemeBuilder) -> opaline::ThemeBuilder {
    b.palette("base", opaline::OpalineColor::new(0x1A, 0x1A, 0x2E))
        .palette("light", opaline::OpalineColor::new(0x16, 0x21, 0x3E))
        .palette("dark", opaline::OpalineColor::new(0x0F, 0x0F, 0x1A))
        .palette("user_accent", opaline::OpalineColor::new(0x6E, 0xE7, 0xB7))
        .palette("thinking_accent", opaline::OpalineColor::new(0xFB, 0xBF, 0x24))
        .palette("tool_accent", opaline::OpalineColor::new(0x60, 0xA5, 0xFA))
        .palette("error_accent", opaline::OpalineColor::new(0xF8, 0x71, 0x71))
        .palette("success_accent", opaline::OpalineColor::new(0x34, 0xD3, 0x99))
}

fn add_grok_night_tokens(b: opaline::ThemeBuilder) -> opaline::ThemeBuilder {
    b.token("bg.base", opaline::OpalineColor::new(0x1A, 0x1A, 0x2E))
        .token("bg.light", opaline::OpalineColor::new(0x16, 0x21, 0x3E))
        .token("bg.dark", opaline::OpalineColor::new(0x0F, 0x0F, 0x1A))
        .token("bg.highlight", opaline::OpalineColor::new(0x25, 0x25, 0x42))
        .token("bg.hover", opaline::OpalineColor::new(0x2F, 0x2F, 0x4F))
        .token("bg.terminal", opaline::OpalineColor::new(0x0A, 0x0A, 0x14))
        .token("bg.panel", opaline::OpalineColor::new(0x20, 0x1F, 0x26))
        .token("text.primary", opaline::OpalineColor::new(0xFF, 0xFA, 0xF1))
        .token("text.secondary", opaline::OpalineColor::new(0xDF, 0xDB, 0xDD))
        .token("text.muted", opaline::OpalineColor::new(0xBF, 0xBC, 0xC8))
        .token("text.dim", opaline::OpalineColor::new(0x8A, 0x87, 0x94))
        .token("accent.primary", opaline::OpalineColor::new(0x6E, 0xE7, 0xB7))
        .token("accent.secondary", opaline::OpalineColor::new(0x60, 0xA5, 0xFA))
        .token("accent.tertiary", opaline::OpalineColor::new(0xFB, 0xBF, 0x24))
        .token("accent.user", opaline::OpalineColor::new(0x6E, 0xE7, 0xB7))
        .token("accent.assistant", opaline::OpalineColor::new(0x60, 0xA5, 0xFA))
        .token("accent.thinking", opaline::OpalineColor::new(0xFB, 0xBF, 0x24))
        .token("accent.tool", opaline::OpalineColor::new(0x60, 0xA5, 0xFA))
        .token("accent.system", opaline::OpalineColor::new(0x3A, 0x39, 0x43))
        .token("accent.error", opaline::OpalineColor::new(0xF8, 0x71, 0x71))
        .token("accent.success", opaline::OpalineColor::new(0x34, 0xD3, 0x99))
        .token("accent.running", opaline::OpalineColor::new(0xFB, 0xBF, 0x24))
        .token("accent.skill", opaline::OpalineColor::new(0x6E, 0xE7, 0xB7))
        .token("accent.plan", opaline::OpalineColor::new(0xFB, 0xBF, 0x24))
        .token("accent.feedback", opaline::OpalineColor::new(0xF8, 0x71, 0x71))
        .token("accent.model", opaline::OpalineColor::new(0x60, 0xA5, 0xFA))
        .token("success", opaline::OpalineColor::new(0x34, 0xD3, 0x99))
        .token("warning", opaline::OpalineColor::new(0xFB, 0xBF, 0x24))
        .token("error", opaline::OpalineColor::new(0xF8, 0x71, 0x71))
        .token("command", opaline::OpalineColor::new(0x6E, 0xE7, 0xB7))
        .token("path", opaline::OpalineColor::new(0x60, 0xA5, 0xFA))
        .token("running", opaline::OpalineColor::new(0xFB, 0xBF, 0x24))
        .token("fuzzy.accent", opaline::OpalineColor::new(0x6E, 0xE7, 0xB7))
        .token("feed.user.bar", opaline::OpalineColor::new(0x6E, 0xE7, 0xB7))
        .token("feed.assistant.bar", opaline::OpalineColor::new(0x60, 0xA5, 0xFA))
        .token("feed.tool.bar", opaline::OpalineColor::new(0x60, 0xA5, 0xFA))
        .token("feed.agent.bar", opaline::OpalineColor::new(0xFB, 0xBF, 0x24))
        .token("feed.system.bar", opaline::OpalineColor::new(0x3A, 0x39, 0x43))
        .token("feed.user.bg", opaline::OpalineColor::new(0x12, 0x12, 0x22))
        .token("feed.separator", opaline::OpalineColor::new(0xBF, 0xBC, 0xC8))
        .token("code.path", opaline::OpalineColor::new(0x6E, 0xE7, 0xB7))
        .token("code.keyword", opaline::OpalineColor::new(0x60, 0xA5, 0xFA))
        .token("code.string", opaline::OpalineColor::new(0x34, 0xD3, 0x99))
        .token("code.comment", opaline::OpalineColor::new(0x8A, 0x87, 0x94))
        .token("code.type", opaline::OpalineColor::new(0xFB, 0xBF, 0x24))
        .token("diff.removed", opaline::OpalineColor::new(0xF8, 0x71, 0x71))
        .token("diff.added", opaline::OpalineColor::new(0x34, 0xD3, 0x99))
        .token("diff.removed_bg", opaline::OpalineColor::new(0x2A, 0x15, 0x15))
        .token("diff.added_bg", opaline::OpalineColor::new(0x15, 0x2A, 0x1A))
        .token("text.plan", opaline::OpalineColor::new(0xFB, 0xBF, 0x24))
        .token("border.unfocused", opaline::OpalineColor::new(0x3A, 0x39, 0x43))
        .token("border.focused", opaline::OpalineColor::new(0x6E, 0xE7, 0xB7))
}

// ─── GrokDay Theme ───────────────────────────────────────────────────────────

fn add_grok_day_palettes(b: opaline::ThemeBuilder) -> opaline::ThemeBuilder {
    b.palette("base", opaline::OpalineColor::new(0xF8, 0xF9, 0xFA))
        .palette("light", opaline::OpalineColor::new(0xFF, 0xFF, 0xFF))
        .palette("dark", opaline::OpalineColor::new(0xE9, 0xEC, 0xEF))
        .palette("user_accent", opaline::OpalineColor::new(0x05, 0x8A, 0x4F))
        .palette("thinking_accent", opaline::OpalineColor::new(0xD4, 0xA0, 0x4E))
        .palette("tool_accent", opaline::OpalineColor::new(0x21, 0x68, 0xC0))
        .palette("error_accent", opaline::OpalineColor::new(0xDC, 0x26, 0x26))
        .palette("success_accent", opaline::OpalineColor::new(0x10, 0xB9, 0x81))
}

fn add_grok_day_tokens(b: opaline::ThemeBuilder) -> opaline::ThemeBuilder {
    b.token("bg.base", opaline::OpalineColor::new(0xF8, 0xF9, 0xFA))
        .token("bg.light", opaline::OpalineColor::new(0xFF, 0xFF, 0xFF))
        .token("bg.dark", opaline::OpalineColor::new(0xE9, 0xEC, 0xEF))
        .token("bg.highlight", opaline::OpalineColor::new(0xF0, 0xF2, 0xF5))
        .token("bg.hover", opaline::OpalineColor::new(0xE9, 0xEC, 0xEF))
        .token("bg.terminal", opaline::OpalineColor::new(0xFF, 0xFF, 0xFF))
        .token("bg.panel", opaline::OpalineColor::new(0xF0, 0xF2, 0xF5))
        .token("text.primary", opaline::OpalineColor::new(0x1F, 0x23, 0x28))
        .token("text.secondary", opaline::OpalineColor::new(0x6C, 0x75, 0x7D))
        .token("text.muted", opaline::OpalineColor::new(0xAD, 0xB1, 0xB8))
        .token("text.dim", opaline::OpalineColor::new(0x9C, 0xA0, 0xA7))
        .token("accent.primary", opaline::OpalineColor::new(0x05, 0x8A, 0x4F))
        .token("accent.secondary", opaline::OpalineColor::new(0x21, 0x68, 0xC0))
        .token("accent.tertiary", opaline::OpalineColor::new(0xD4, 0xA0, 0x4E))
        .token("accent.user", opaline::OpalineColor::new(0x05, 0x8A, 0x4F))
        .token("accent.assistant", opaline::OpalineColor::new(0x21, 0x68, 0xC0))
        .token("accent.thinking", opaline::OpalineColor::new(0xD4, 0xA0, 0x4E))
        .token("accent.tool", opaline::OpalineColor::new(0x21, 0x68, 0xC0))
        .token("accent.system", opaline::OpalineColor::new(0x6C, 0x75, 0x7D))
        .token("accent.error", opaline::OpalineColor::new(0xDC, 0x26, 0x26))
        .token("accent.success", opaline::OpalineColor::new(0x10, 0xB9, 0x81))
        .token("accent.running", opaline::OpalineColor::new(0xD4, 0xA0, 0x4E))
        .token("accent.skill", opaline::OpalineColor::new(0x05, 0x8A, 0x4F))
        .token("accent.plan", opaline::OpalineColor::new(0xD4, 0xA0, 0x4E))
        .token("accent.feedback", opaline::OpalineColor::new(0xDC, 0x26, 0x26))
        .token("accent.model", opaline::OpalineColor::new(0x21, 0x68, 0xC0))
        .token("success", opaline::OpalineColor::new(0x10, 0xB9, 0x81))
        .token("warning", opaline::OpalineColor::new(0xD4, 0xA0, 0x4E))
        .token("error", opaline::OpalineColor::new(0xDC, 0x26, 0x26))
        .token("command", opaline::OpalineColor::new(0x05, 0x8A, 0x4F))
        .token("path", opaline::OpalineColor::new(0x21, 0x68, 0xC0))
        .token("running", opaline::OpalineColor::new(0xD4, 0xA0, 0x4E))
        .token("fuzzy.accent", opaline::OpalineColor::new(0x05, 0x8A, 0x4F))
        .token("feed.user.bar", opaline::OpalineColor::new(0x05, 0x8A, 0x4F))
        .token("feed.assistant.bar", opaline::OpalineColor::new(0x21, 0x68, 0xC0))
        .token("feed.tool.bar", opaline::OpalineColor::new(0x21, 0x68, 0xC0))
        .token("feed.agent.bar", opaline::OpalineColor::new(0xD4, 0xA0, 0x4E))
        .token("feed.system.bar", opaline::OpalineColor::new(0x6C, 0x75, 0x7D))
        .token("feed.user.bg", opaline::OpalineColor::new(0xF0, 0xF2, 0xF5))
        .token("feed.separator", opaline::OpalineColor::new(0xAD, 0xB1, 0xB8))
        .token("code.path", opaline::OpalineColor::new(0x05, 0x8A, 0x4F))
        .token("code.keyword", opaline::OpalineColor::new(0x21, 0x68, 0xC0))
        .token("code.string", opaline::OpalineColor::new(0x10, 0xB9, 0x81))
        .token("code.comment", opaline::OpalineColor::new(0x9C, 0xA0, 0xA7))
        .token("code.type", opaline::OpalineColor::new(0xD4, 0xA0, 0x4E))
        .token("diff.removed", opaline::OpalineColor::new(0xDC, 0x26, 0x26))
        .token("diff.added", opaline::OpalineColor::new(0x10, 0xB9, 0x81))
        .token("diff.removed_bg", opaline::OpalineColor::new(0xFD, 0xE2, 0xE2))
        .token("diff.added_bg", opaline::OpalineColor::new(0xD5, 0xF5, 0xE3))
        .token("text.plan", opaline::OpalineColor::new(0xD4, 0xA0, 0x4E))
        .token("border.unfocused", opaline::OpalineColor::new(0xAD, 0xB1, 0xB8))
        .token("border.focused", opaline::OpalineColor::new(0x05, 0x8A, 0x4F))
}

// ─── TokyoNight Theme ────────────────────────────────────────────────────────

fn add_tokyo_night_palettes(b: opaline::ThemeBuilder) -> opaline::ThemeBuilder {
    b.palette("base", opaline::OpalineColor::new(0x1A, 0x1B, 0x26))
        .palette("light", opaline::OpalineColor::new(0x24, 0x28, 0x3B))
        .palette("dark", opaline::OpalineColor::new(0x16, 0x16, 0x1E))
        .palette("user_accent", opaline::OpalineColor::new(0x7A, 0xA2, 0xF7))
        .palette("thinking_accent", opaline::OpalineColor::new(0xBB, 0x9A, 0xF7))
        .palette("tool_accent", opaline::OpalineColor::new(0x9E, 0xCE, 0x6A))
        .palette("error_accent", opaline::OpalineColor::new(0xF7, 0x76, 0x8E))
        .palette("success_accent", opaline::OpalineColor::new(0x9E, 0xCE, 0x6A))
}

fn add_tokyo_night_tokens(b: opaline::ThemeBuilder) -> opaline::ThemeBuilder {
    b.token("bg.base", opaline::OpalineColor::new(0x1A, 0x1B, 0x26))
        .token("bg.light", opaline::OpalineColor::new(0x24, 0x28, 0x3B))
        .token("bg.dark", opaline::OpalineColor::new(0x16, 0x16, 0x1E))
        .token("bg.highlight", opaline::OpalineColor::new(0x2A, 0x2E, 0x40))
        .token("bg.hover", opaline::OpalineColor::new(0x33, 0x38, 0x4D))
        .token("bg.terminal", opaline::OpalineColor::new(0x0E, 0x0F, 0x17))
        .token("bg.panel", opaline::OpalineColor::new(0x20, 0x1F, 0x26))
        .token("text.primary", opaline::OpalineColor::new(0xC0, 0xCA, 0xD7))
        .token("text.secondary", opaline::OpalineColor::new(0xA9, 0xB1, 0xC6))
        .token("text.muted", opaline::OpalineColor::new(0x56, 0x5F, 0x73))
        .token("text.dim", opaline::OpalineColor::new(0x44, 0x4D, 0x5E))
        .token("accent.primary", opaline::OpalineColor::new(0x7A, 0xA2, 0xF7))
        .token("accent.secondary", opaline::OpalineColor::new(0x9E, 0xCE, 0x6A))
        .token("accent.tertiary", opaline::OpalineColor::new(0xBB, 0x9A, 0xF7))
        .token("accent.user", opaline::OpalineColor::new(0x7A, 0xA2, 0xF7))
        .token("accent.assistant", opaline::OpalineColor::new(0x9E, 0xCE, 0x6A))
        .token("accent.thinking", opaline::OpalineColor::new(0xBB, 0x9A, 0xF7))
        .token("accent.tool", opaline::OpalineColor::new(0x9E, 0xCE, 0x6A))
        .token("accent.system", opaline::OpalineColor::new(0x41, 0x47, 0x5E))
        .token("accent.error", opaline::OpalineColor::new(0xF7, 0x76, 0x8E))
        .token("accent.success", opaline::OpalineColor::new(0x9E, 0xCE, 0x6A))
        .token("accent.running", opaline::OpalineColor::new(0x7A, 0xA2, 0xF7))
        .token("accent.skill", opaline::OpalineColor::new(0xBB, 0x9A, 0xF7))
        .token("accent.plan", opaline::OpalineColor::new(0xBB, 0x9A, 0xF7))
        .token("accent.feedback", opaline::OpalineColor::new(0xF7, 0x76, 0x8E))
        .token("accent.model", opaline::OpalineColor::new(0x9E, 0xCE, 0x6A))
        .token("success", opaline::OpalineColor::new(0x9E, 0xCE, 0x6A))
        .token("warning", opaline::OpalineColor::new(0xFF, 0x9E, 0x64))
        .token("error", opaline::OpalineColor::new(0xF7, 0x76, 0x8E))
        .token("command", opaline::OpalineColor::new(0x7A, 0xA2, 0xF7))
        .token("path", opaline::OpalineColor::new(0x9E, 0xCE, 0x6A))
        .token("running", opaline::OpalineColor::new(0x7A, 0xA2, 0xF7))
        .token("fuzzy.accent", opaline::OpalineColor::new(0x7A, 0xA2, 0xF7))
        .token("feed.user.bar", opaline::OpalineColor::new(0x7A, 0xA2, 0xF7))
        .token("feed.assistant.bar", opaline::OpalineColor::new(0x9E, 0xCE, 0x6A))
        .token("feed.tool.bar", opaline::OpalineColor::new(0x9E, 0xCE, 0x6A))
        .token("feed.agent.bar", opaline::OpalineColor::new(0xBB, 0x9A, 0xF7))
        .token("feed.system.bar", opaline::OpalineColor::new(0x41, 0x47, 0x5E))
        .token("feed.user.bg", opaline::OpalineColor::new(0x12, 0x13, 0x1E))
        .token("feed.separator", opaline::OpalineColor::new(0x56, 0x5F, 0x73))
        .token("code.path", opaline::OpalineColor::new(0x7A, 0xA2, 0xF7))
        .token("code.keyword", opaline::OpalineColor::new(0xBB, 0x9A, 0xF7))
        .token("code.string", opaline::OpalineColor::new(0x9E, 0xCE, 0x6A))
        .token("code.comment", opaline::OpalineColor::new(0x56, 0x5F, 0x73))
        .token("code.type", opaline::OpalineColor::new(0xFF, 0x9E, 0x64))
        .token("diff.removed", opaline::OpalineColor::new(0xF7, 0x76, 0x8E))
        .token("diff.added", opaline::OpalineColor::new(0x9E, 0xCE, 0x6A))
        .token("diff.removed_bg", opaline::OpalineColor::new(0x2A, 0x15, 0x1E))
        .token("diff.added_bg", opaline::OpalineColor::new(0x1A, 0x2A, 0x15))
        .token("text.plan", opaline::OpalineColor::new(0xBB, 0x9A, 0xF7))
        .token("border.unfocused", opaline::OpalineColor::new(0x41, 0x47, 0x5E))
        .token("border.focused", opaline::OpalineColor::new(0x7A, 0xA2, 0xF7))
}

// ─── RosePineMoon Theme ───────────────────────────────────────────────────────

fn add_rose_pine_moon_palettes(b: opaline::ThemeBuilder) -> opaline::ThemeBuilder {
    b.palette("base", opaline::OpalineColor::new(0x23, 0x21, 0x36))
        .palette("light", opaline::OpalineColor::new(0x2A, 0x27, 0x40))
        .palette("dark", opaline::OpalineColor::new(0x1F, 0x1A, 0x2E))
        .palette("user_accent", opaline::OpalineColor::new(0xC4, 0xA7, 0xE7))
        .palette("thinking_accent", opaline::OpalineColor::new(0xF6, 0xC1, 0x77))
        .palette("tool_accent", opaline::OpalineColor::new(0xEB, 0xBC, 0xBA))
        .palette("error_accent", opaline::OpalineColor::new(0xEB, 0x6F, 0x92))
        .palette("success_accent", opaline::OpalineColor::new(0x9C, 0xCF, 0xD0))
}

fn add_rose_pine_moon_tokens(b: opaline::ThemeBuilder) -> opaline::ThemeBuilder {
    b.token("bg.base", opaline::OpalineColor::new(0x23, 0x21, 0x36))
        .token("bg.light", opaline::OpalineColor::new(0x2A, 0x27, 0x40))
        .token("bg.dark", opaline::OpalineColor::new(0x1F, 0x1A, 0x2E))
        .token("bg.highlight", opaline::OpalineColor::new(0x31, 0x2D, 0x48))
        .token("bg.hover", opaline::OpalineColor::new(0x3A, 0x35, 0x52))
        .token("bg.terminal", opaline::OpalineColor::new(0x17, 0x15, 0x24))
        .token("bg.panel", opaline::OpalineColor::new(0x2A, 0x27, 0x40))
        .token("text.primary", opaline::OpalineColor::new(0xE0, 0xDC, 0xF2))
        .token("text.secondary", opaline::OpalineColor::new(0xC4, 0xA7, 0xE7))
        .token("text.muted", opaline::OpalineColor::new(0x90, 0x7B, 0xAA))
        .token("text.dim", opaline::OpalineColor::new(0x73, 0x6A, 0x8A))
        .token("accent.primary", opaline::OpalineColor::new(0xC4, 0xA7, 0xE7))
        .token("accent.secondary", opaline::OpalineColor::new(0xEB, 0xBC, 0xBA))
        .token("accent.tertiary", opaline::OpalineColor::new(0xF6, 0xC1, 0x77))
        .token("accent.user", opaline::OpalineColor::new(0xC4, 0xA7, 0xE7))
        .token("accent.assistant", opaline::OpalineColor::new(0xEB, 0xBC, 0xBA))
        .token("accent.thinking", opaline::OpalineColor::new(0xF6, 0xC1, 0x77))
        .token("accent.tool", opaline::OpalineColor::new(0xEB, 0xBC, 0xBA))
        .token("accent.system", opaline::OpalineColor::new(0x39, 0x36, 0x4E))
        .token("accent.error", opaline::OpalineColor::new(0xEB, 0x6F, 0x92))
        .token("accent.success", opaline::OpalineColor::new(0x9C, 0xCF, 0xD0))
        .token("accent.running", opaline::OpalineColor::new(0xC4, 0xA7, 0xE7))
        .token("accent.skill", opaline::OpalineColor::new(0xF6, 0xC1, 0x77))
        .token("accent.plan", opaline::OpalineColor::new(0xF6, 0xC1, 0x77))
        .token("accent.feedback", opaline::OpalineColor::new(0xEB, 0x6F, 0x92))
        .token("accent.model", opaline::OpalineColor::new(0xEB, 0xBC, 0xBA))
        .token("success", opaline::OpalineColor::new(0x9C, 0xCF, 0xD0))
        .token("warning", opaline::OpalineColor::new(0xF6, 0xC1, 0x77))
        .token("error", opaline::OpalineColor::new(0xEB, 0x6F, 0x92))
        .token("command", opaline::OpalineColor::new(0xC4, 0xA7, 0xE7))
        .token("path", opaline::OpalineColor::new(0xEB, 0xBC, 0xBA))
        .token("running", opaline::OpalineColor::new(0xC4, 0xA7, 0xE7))
        .token("fuzzy.accent", opaline::OpalineColor::new(0xC4, 0xA7, 0xE7))
        .token("feed.user.bar", opaline::OpalineColor::new(0xC4, 0xA7, 0xE7))
        .token("feed.assistant.bar", opaline::OpalineColor::new(0xEB, 0xBC, 0xBA))
        .token("feed.tool.bar", opaline::OpalineColor::new(0xEB, 0xBC, 0xBA))
        .token("feed.agent.bar", opaline::OpalineColor::new(0xF6, 0xC1, 0x77))
        .token("feed.system.bar", opaline::OpalineColor::new(0x39, 0x36, 0x4E))
        .token("feed.user.bg", opaline::OpalineColor::new(0x1A, 0x18, 0x28))
        .token("feed.separator", opaline::OpalineColor::new(0x90, 0x7B, 0xAA))
        .token("code.path", opaline::OpalineColor::new(0xC4, 0xA7, 0xE7))
        .token("code.keyword", opaline::OpalineColor::new(0xEB, 0x6F, 0x92))
        .token("code.string", opaline::OpalineColor::new(0x9C, 0xCF, 0xD0))
        .token("code.comment", opaline::OpalineColor::new(0x73, 0x6A, 0x8A))
        .token("code.type", opaline::OpalineColor::new(0xF6, 0xC1, 0x77))
        .token("diff.removed", opaline::OpalineColor::new(0xEB, 0x6F, 0x92))
        .token("diff.added", opaline::OpalineColor::new(0x9C, 0xCF, 0xD0))
        .token("diff.removed_bg", opaline::OpalineColor::new(0x2A, 0x15, 0x20))
        .token("diff.added_bg", opaline::OpalineColor::new(0x1A, 0x2A, 0x28))
        .token("text.plan", opaline::OpalineColor::new(0xF6, 0xC1, 0x77))
        .token("border.unfocused", opaline::OpalineColor::new(0x39, 0x36, 0x4E))
        .token("border.focused", opaline::OpalineColor::new(0xC4, 0xA7, 0xE7))
}

// ─── Extra Tokens ─────────────────────────────────────────────────────────────

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

// ─── ThemeColors ─────────────────────────────────────────────────────────────

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
            bg_base: theme.color("bg.base").into(),
            bg_panel: theme.color("bg.panel").into(),
            accent_primary: theme.color("accent.primary").into(),
            accent_secondary: theme.color("accent.secondary").into(),
            text_primary: theme.color("text.primary").into(),
            text_secondary: theme.color("text.secondary").into(),
            text_dim: theme.color("text.dim").into(),
            text_muted: theme.color("text.muted").into(),
            border_unfocused: theme.color("border.unfocused").into(),
            success: theme.color("success").into(),
            error: theme.color("error").into(),
            warning: theme.color("warning").into(),
            syntax_phase: theme.color("syntax.phase").into(),
            text_plan: theme.color("text.plan").into(),
            feed_tool_bar: theme.color("feed.tool.bar").into(),
            accent_user: theme.color("accent.user").into(),
            accent_assistant: theme.color("accent.assistant").into(),
            accent_thinking: theme.color("accent.thinking").into(),
            accent_tool: theme.color("accent.tool").into(),
            accent_system: theme.color("accent.system").into(),
            accent_error: theme.color("accent.error").into(),
            accent_success: theme.color("accent.success").into(),
            accent_running: theme.color("accent.running").into(),
            accent_skill: theme.color("accent.skill").into(),
            accent_plan: theme.color("accent.plan").into(),
            accent_feedback: theme.color("accent.feedback").into(),
            accent_model: theme.color("accent.model").into(),
            accent_teal: theme.color("accent.teal").into(),
            accent_orange: theme.color("accent.orange").into(),
            accent_purple: theme.color("accent.purple").into(),
            accent_yellow: theme.color("accent.yellow").into(),
            accent_blue_bright: theme.color("accent.blue_bright").into(),
            command: theme.color("command").into(),
            path: theme.color("path").into(),
            running: theme.color("running").into(),
            fuzzy_accent: theme.color("fuzzy.accent").into(),
            editor_bg: theme.color("editor.bg").into(),
            surface_bg: theme.color("surface.bg").into(),
            popover_bg: theme.color("popover.bg").into(),
        }
    }
}
    }
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
