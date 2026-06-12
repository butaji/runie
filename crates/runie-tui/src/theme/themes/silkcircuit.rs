//! Silkcircuit neon theme builder (uses opaline builtins).

use opaline::Theme;

pub fn build_silkcircuit() -> Theme {
    Theme::from(opaline::builtins::silkcircuit_neon())
}
