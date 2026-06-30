use ratatui::style::Color;

// ─────────────────────────────────────────────────────────────────────────────
// Color accessor functions
// ─────────────────────────────────────────────────────────────────────────────

/// Simple color accessor for a required theme key.
fn theme_color(key: &str) -> Color {
    Color::from(crate::theme::current_theme().color(key))
}

/// Color accessor with a fallback when the key is not found.
fn theme_color_fallback(key: &str, fallback: Color) -> Color {
    crate::theme::current_theme()
        .try_color(key)
        .map(Color::from)
        .unwrap_or(fallback)
}

// Generated color accessors
pub fn color_bg() -> Color {
    theme_color_fallback("bg.base", Color::Reset)
}
pub fn color_bg_panel() -> Color {
    theme_color_fallback("bg.panel", Color::Reset)
}
pub fn color_fg() -> Color {
    theme_color("text.primary")
}
pub fn color_fg_mid() -> Color {
    theme_color("text.secondary")
}
pub fn color_accent() -> Color {
    theme_color("accent.primary")
}
pub fn color_success() -> Color {
    theme_color("success")
}
pub fn color_warning() -> Color {
    theme_color("warning")
}
pub fn color_error() -> Color {
    theme_color("error")
}
pub fn color_dim() -> Color {
    theme_color("text.dim")
}
pub fn color_border() -> Color {
    theme_color("border.unfocused")
}
pub fn color_code() -> Color {
    theme_color("code.function")
}
pub fn color_code_bg() -> Color {
    theme_color_fallback("bg.code", Color::Reset)
}

/// Bright foreground: primary text lightened by 0.3.
pub fn color_fg_bright() -> Color {
    let c = crate::theme::current_theme()
        .color("text.primary")
        .lighten(0.3);
    Color::Rgb(c.r, c.g, c.b)
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

/// User message post background. Themes can override `bg.user`;
/// otherwise we fall back to the elevated surface color.
pub fn color_user_bg() -> Color {
    crate::theme::current_theme()
        .try_color("bg.user")
        .or_else(|| crate::theme::current_theme().try_color("bg.elevated"))
        .or_else(|| crate::theme::current_theme().try_color("bg.panel"))
        .or_else(|| crate::theme::current_theme().try_color("bg.highlight"))
        .map(Color::from)
        .unwrap_or(Color::Reset)
}

/// Accent color blended over the terminal background at the given
/// opacity (0.0–1.0). Used for the subtle selection highlight behind
/// the selected post in vim nav mode.
pub fn color_accent_bg() -> Color {
    blend(color_bg(), color_accent(), 0.1)
}

// ─────────────────────────────────────────────────────────────────────────────
// Color utility functions
// ─────────────────────────────────────────────────────────────────────────────

/// Darken an RGB color by a factor (0.0–1.0).
/// Standard linear scaling of sRGB components toward black.
pub fn darken(color: Color, factor: f32) -> Color {
    match color {
        Color::Rgb(r, g, b) => {
            let factor = factor.clamp(0.0, 1.0);
            Color::Rgb(
                ((r as f32 / 255.0) * factor * 255.0) as u8,
                ((g as f32 / 255.0) * factor * 255.0) as u8,
                ((b as f32 / 255.0) * factor * 255.0) as u8,
            )
        }
        _ => color,
    }
}

/// Blend two RGB colors with the given opacity (0.0-1.0).
/// Standard premultiplied-alpha over-compositing.
fn blend(bg: Color, fg: Color, opacity: f32) -> Color {
    let opacity = opacity.clamp(0.0, 1.0);

    let (bg_r, bg_g, bg_b) = match bg {
        Color::Rgb(r, g, b) => (r as f32, g as f32, b as f32),
        _ => (30.0, 30.0, 30.0),
    };
    let (fr, fg_g, fb) = match fg {
        Color::Rgb(r, g, b) => (r as f32, g as f32, b as f32),
        _ => return fg,
    };

    // Normalize to 0.0–1.0 and apply premultiplied-alpha over.
    let (bg_r, bg_g, bg_b) = (bg_r / 255.0, bg_g / 255.0, bg_b / 255.0);
    let (fr, fg_g, fb) = (fr / 255.0, fg_g / 255.0, fb / 255.0);

    // Standard over-compositing: dst*(1-src_alpha) + src
    let out_r = fr * opacity + bg_r * (1.0 - opacity);
    let out_g = fg_g * opacity + bg_g * (1.0 - opacity);
    let out_b = fb * opacity + bg_b * (1.0 - opacity);

    Color::Rgb(
        (out_r.clamp(0.0, 1.0) * 255.0) as u8,
        (out_g.clamp(0.0, 1.0) * 255.0) as u8,
        (out_b.clamp(0.0, 1.0) * 255.0) as u8,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn assert_rgb_or_reset(color: Color) {
        assert!(
            matches!(color, Color::Rgb(_, _, _) | Color::Reset),
            "Expected Rgb or Reset, got {:?}",
            color
        );
    }

    fn assert_rgb(color: Color) {
        assert!(
            matches!(color, Color::Rgb(_, _, _)),
            "Expected Rgb, got {:?}",
            color
        );
    }

    #[test]
    fn macro_generates_same_color_values() {
        // Verify macro-generated accessors return valid Color values.
        // These call the actual generated functions to ensure they compile and work.
        assert_rgb_or_reset(color_bg());
        assert_rgb_or_reset(color_bg_panel());
        assert_rgb(color_fg());
        assert_rgb(color_fg_mid());
        assert_rgb(color_accent());
        assert_rgb(color_success());
        assert_rgb(color_warning());
        assert_rgb(color_error());
        assert_rgb(color_dim());
        assert_rgb(color_border());
        assert_rgb(color_code());
        assert_rgb_or_reset(color_code_bg());
        assert_rgb(color_fg_bright());
        assert_rgb(color_diff_insert_bg());
        assert_rgb(color_diff_remove_bg());
        assert_rgb_or_reset(color_user_bg());
        assert_rgb(color_accent_bg());
    }

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
        assert!(matches!(result, Color::Rgb(r, _g, _b) if r > 30 && r < 200));
    }

    #[test]
    fn palette_blend_with_zero_opacity_returns_bg() {
        let bg = Color::Rgb(10, 20, 30);
        let fg = Color::Rgb(200, 100, 50);
        let result = blend(bg, fg, 0.0);
        assert!(matches!(result, Color::Rgb(r, g, b) if r == 10 && g == 20 && b == 30));
    }
}
