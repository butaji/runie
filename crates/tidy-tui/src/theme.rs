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

    pub fn crush_grok() -> Self {
        use opaline::{Theme, OpalineColor, ThemeVariant};

        let theme = Theme::builder("crush-grok")
            .variant(ThemeVariant::Dark)
            .author("Tidy")
            .description("Crush + GrokBuild hybrid: Charm glam meets cosmic truth-seeking")
            // Core palette
            .palette("pepper", OpalineColor::new(0x20, 0x1F, 0x26))       // #201F26
            .palette("cosmic", OpalineColor::new(0x0F, 0x0C, 0x14))       // #0F0C14
            .palette("charple", OpalineColor::new(0x6B, 0x50, 0xFF))      // #6B50FF
            .palette("grok_orange", OpalineColor::new(0xFF, 0x6B, 0x00))  // #FF6B00
            .palette("neon_teal", OpalineColor::new(0x00, 0xF5, 0xD4))    // #00F5D4
            .palette("dolly_pink", OpalineColor::new(0xFF, 0x60, 0xFF))   // #FF60FF
            .palette("butter", OpalineColor::new(0xFF, 0xFA, 0xF1))       // #FFFAF1
            .palette("ash", OpalineColor::new(0xDF, 0xDB, 0xDD))         // #DFDBDD
            .palette("smoke", OpalineColor::new(0xBF, 0xBC, 0xC8))       // #BFBCC8
            .palette("sriracha", OpalineColor::new(0xEB, 0x42, 0x68))    // #EB4268
            .palette("charcoal", OpalineColor::new(0x3A, 0x39, 0x43))    // #3A3943
            // Semantic tokens
            .token("bg.base", OpalineColor::new(0x0F, 0x0C, 0x14))        // cosmic
            .token("bg.panel", OpalineColor::new(0x20, 0x1F, 0x26))       // pepper
            .token("text.primary", OpalineColor::new(0xFF, 0xFA, 0xF1))   // butter
            .token("text.secondary", OpalineColor::new(0xDF, 0xDB, 0xDD)) // ash
            .token("text.muted", OpalineColor::new(0xBF, 0xBC, 0xC8))     // smoke
            .token("text.dim", OpalineColor::new(0x8A, 0x87, 0x94))       // darker smoke
            .token("accent.primary", OpalineColor::new(0xFF, 0x6B, 0x00)) // grok_orange
            .token("accent.secondary", OpalineColor::new(0x00, 0xF5, 0xD4)) // neon_teal
            .token("accent.tertiary", OpalineColor::new(0xFF, 0x60, 0xFF)) // dolly_pink
            .token("success", OpalineColor::new(0x00, 0xF5, 0xD4))        // neon_teal
            .token("warning", OpalineColor::new(0xFF, 0x6B, 0x00))        // grok_orange
            .token("error", OpalineColor::new(0xEB, 0x42, 0x68))          // sriracha
            .token("border.unfocused", OpalineColor::new(0x3A, 0x39, 0x43)) // charcoal
            .token("border.focused", OpalineColor::new(0xFF, 0x6B, 0x00))  // grok_orange
            // Feed-specific tokens (Crush+Grok design)
            .token("feed.user.bar", OpalineColor::new(0xFF, 0x6B, 0x00))   // grok_orange
            .token("feed.assistant.bar", OpalineColor::new(0x00, 0xF5, 0xD4)) // neon_teal
            .token("feed.tool.bar", OpalineColor::new(0x6B, 0x50, 0xFF))   // charple
            .token("feed.agent.bar", OpalineColor::new(0xFF, 0x60, 0xFF))  // dolly_pink
            .token("feed.system.bar", OpalineColor::new(0x3A, 0x39, 0x43)) // charcoal
            .token("feed.user.bg", OpalineColor::new(0x1A, 0x19, 0x20))   // slightly lighter than pepper
            .token("feed.separator", OpalineColor::new(0xBF, 0xBC, 0xC8)) // smoke
            // Code/syntax tokens
            .token("code.path", OpalineColor::new(0x6B, 0x50, 0xFF))      // charple
            .token("code.keyword", OpalineColor::new(0x00, 0xF5, 0xD4))   // neon_teal
            .token("code.string", OpalineColor::new(0xFF, 0x60, 0xFF))    // dolly_pink
            .token("code.comment", OpalineColor::new(0x8A, 0x87, 0x94))   // dim smoke
            .token("code.type", OpalineColor::new(0x39, 0xFF, 0x8C))      // mint green
            .build();

        Self { inner: theme }
    }
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

// Alias for backwards compatibility
pub type ColorPalette = ThemeWrapper;

// Re-export for lib.rs
pub use opaline::OpalineColor;
