//! Crush Grok theme builder.

use opaline::{Theme, ThemeVariant};
use crate::theme::themes::apply_extra_tokens;

pub fn build_crush_grok() -> Theme {
    let mut b = Theme::builder("crush-grok")
        .variant(ThemeVariant::Dark)
        .author("Runie")
        .description("Crush + GrokBuild hybrid");
    b = add_crush_grok_palettes(b);
    b = add_crush_grok_tokens(b);
    b = add_crush_grok_feed_tokens(b);
    b = add_crush_grok_code_tokens(b);
    b = add_crush_grok_diff_tokens(b);
    apply_extra_tokens(b).build()
}

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
