use opaline::Theme;

#[derive(Debug, Clone)]
pub struct ThemeWrapper {
    inner: Theme,
}

impl ThemeWrapper {
    pub fn color(&self, token: &str) -> opaline::OpalineColor {
        self.inner.color(token)
    }

    /// Creates a default ThemeWrapper for testing.
    #[cfg(test)]
    pub fn default_for_test() -> Self {
        Self::default()
    }

    pub fn silkcircuit_neon() -> Self {
        Self { inner: Theme::from(opaline::builtins::silkcircuit_neon()) }
    }

    pub fn crush_grok() -> Self {
        use opaline::{Theme, ThemeVariant};
        let mut b = Theme::builder("crush-grok")
            .variant(ThemeVariant::Dark)
            .author("Runie")
            .description("Crush + GrokBuild hybrid");
        b = add_core_palettes(b);
        b = add_semantic_tokens(b);
        b = add_feed_tokens(b);
        b = add_code_tokens(b);
        b = add_diff_tokens(b);
        Self { inner: b.build() }
    }
}

fn add_core_palettes(b: opaline::ThemeBuilder) -> opaline::ThemeBuilder {
    use opaline::OpalineColor;
    b.palette("pepper", OpalineColor::new(0x20, 0x1F, 0x26))
        .palette("cosmic", OpalineColor::new(0x0F, 0x0C, 0x14))
        .palette("charple", OpalineColor::new(0x6B, 0x50, 0xFF))
        .palette("grok_orange", OpalineColor::new(0xFF, 0x6B, 0x00))
        .palette("neon_teal", OpalineColor::new(0x00, 0xF5, 0xD4))
        .palette("dolly_pink", OpalineColor::new(0xFF, 0x60, 0xFF))
        .palette("butter", OpalineColor::new(0xFF, 0xFA, 0xF1))
        .palette("ash", OpalineColor::new(0xDF, 0xDB, 0xDD))
        .palette("smoke", OpalineColor::new(0xBF, 0xBC, 0xC8))
        .palette("sriracha", OpalineColor::new(0xEB, 0x42, 0x68))
        .palette("charcoal", OpalineColor::new(0x3A, 0x39, 0x43))
}

fn add_semantic_tokens(b: opaline::ThemeBuilder) -> opaline::ThemeBuilder {
    use opaline::OpalineColor;
    b.token("bg.base", OpalineColor::new(0x0F, 0x0C, 0x14))
        .token("bg.panel", OpalineColor::new(0x20, 0x1F, 0x26))
        .token("text.primary", OpalineColor::new(0xFF, 0xFA, 0xF1))
        .token("text.secondary", OpalineColor::new(0xDF, 0xDB, 0xDD))
        .token("text.muted", OpalineColor::new(0xBF, 0xBC, 0xC8))
        .token("text.dim", OpalineColor::new(0x8A, 0x87, 0x94))
        .token("accent.primary", OpalineColor::new(0xFF, 0x6B, 0x00))
        .token("accent.secondary", OpalineColor::new(0x00, 0xF5, 0xD4))
        .token("accent.tertiary", OpalineColor::new(0xFF, 0x60, 0xFF))
        .token("success", OpalineColor::new(0x00, 0xF5, 0xD4))
        .token("warning", OpalineColor::new(0xFF, 0x6B, 0x00))
        .token("error", OpalineColor::new(0xEB, 0x42, 0x68))
        .token("border.unfocused", OpalineColor::new(0x3A, 0x39, 0x43))
        .token("border.focused", OpalineColor::new(0xFF, 0x6B, 0x00))
}

fn add_feed_tokens(b: opaline::ThemeBuilder) -> opaline::ThemeBuilder {
    use opaline::OpalineColor;
    b.token("feed.user.bar", OpalineColor::new(0xFF, 0x6B, 0x00))
        .token("feed.assistant.bar", OpalineColor::new(0x00, 0xF5, 0xD4))
        .token("feed.tool.bar", OpalineColor::new(0x6B, 0x50, 0xFF))
        .token("feed.agent.bar", OpalineColor::new(0xFF, 0x60, 0xFF))
        .token("feed.system.bar", OpalineColor::new(0x3A, 0x39, 0x43))
        .token("feed.user.bg", OpalineColor::new(0x1A, 0x19, 0x20))
        .token("feed.separator", OpalineColor::new(0xBF, 0xBC, 0xC8))
}

fn add_code_tokens(b: opaline::ThemeBuilder) -> opaline::ThemeBuilder {
    use opaline::OpalineColor;
    b.token("code.path", OpalineColor::new(0x6B, 0x50, 0xFF))
        .token("code.keyword", OpalineColor::new(0x00, 0xF5, 0xD4))
        .token("code.string", OpalineColor::new(0xFF, 0x60, 0xFF))
        .token("code.comment", OpalineColor::new(0x8A, 0x87, 0x94))
        .token("code.type", OpalineColor::new(0x39, 0xFF, 0x8C))
}

fn add_diff_tokens(b: opaline::ThemeBuilder) -> opaline::ThemeBuilder {
    use opaline::OpalineColor;
    b.token("diff.removed", OpalineColor::new(0xFF, 0x6B, 0x6B))
        .token("diff.added", OpalineColor::new(0x51, 0xCF, 0x66))
        .token("diff.removed_bg", OpalineColor::new(0x3A, 0x1A, 0x1A))
        .token("diff.added_bg", OpalineColor::new(0x1A, 0x3A, 0x1A))
}

impl Default for ThemeWrapper {
    fn default() -> Self {
        Self::crush_grok()
    }
}

impl From<Theme> for ThemeWrapper {
    fn from(inner: Theme) -> Self {
        Self { inner }
    }
}

/// Pre-extracted theme colors for hot-path rendering.
/// Avoids repeated ThemeWrapper lookups per frame.
#[derive(Debug, Clone)]
pub struct ThemeColors {
    pub bg_base: ratatui::style::Color,
    pub bg_panel: ratatui::style::Color,
    pub accent_primary: ratatui::style::Color,
    pub text_primary: ratatui::style::Color,
    pub text_secondary: ratatui::style::Color,
    pub text_dim: ratatui::style::Color,
    pub text_muted: ratatui::style::Color,
    pub border_unfocused: ratatui::style::Color,
    pub success: ratatui::style::Color,
    pub error: ratatui::style::Color,
    pub syntax_phase: ratatui::style::Color,
}

impl From<&ThemeWrapper> for ThemeColors {
    fn from(theme: &ThemeWrapper) -> Self {
        Self {
            bg_base: theme.color("bg.base").into(),
            bg_panel: theme.color("bg.panel").into(),
            accent_primary: theme.color("accent.primary").into(),
            text_primary: theme.color("text.primary").into(),
            text_secondary: theme.color("text.secondary").into(),
            text_dim: theme.color("text.dim").into(),
            text_muted: theme.color("text.muted").into(),
            border_unfocused: theme.color("border.unfocused").into(),
            success: theme.color("success").into(),
            error: theme.color("error").into(),
            syntax_phase: theme.color("syntax.phase").into(),
        }
    }
}

// Alias for backwards compatibility
pub type ColorPalette = ThemeWrapper;

// Re-export for lib.rs
pub use opaline::OpalineColor;
