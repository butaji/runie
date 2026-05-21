use opaline::Theme;

#[derive(Debug, Clone)]
pub struct ThemeWrapper {
    inner: Theme,
}

impl ThemeWrapper {
    pub fn color(&self, token: &str) -> opaline::OpalineColor {
        self.inner.color(token)
    }

    pub fn silkcircuit_neon() -> Self {
        Self { inner: Theme::from(opaline::builtins::silkcircuit_neon()) }
    }
}

impl Default for ThemeWrapper {
    fn default() -> Self {
        Self::silkcircuit_neon()
    }
}

impl From<Theme> for ThemeWrapper {
    fn from(inner: Theme) -> Self {
        Self { inner }
    }
}

// Alias for backwards compatibility
pub type ColorPalette = ThemeWrapper;

// Re-export for lib.rs
pub use opaline::OpalineColor;
