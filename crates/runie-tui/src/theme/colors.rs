use ratatui::style::Color;

// ─────────────────────────────────────────────────────────────────────────────
// Macro-based color accessors
// ─────────────────────────────────────────────────────────────────────────────

/// Generates a simple color accessor from a theme key.
/// Example: `theme_color!(color_fg, "text.primary");`
macro_rules! theme_color {
    ($fn_name:ident, $key:literal) => {
        pub fn $fn_name() -> Color {
            Color::from(crate::theme::current_theme().color($key))
        }
    };
}

/// Generates a color accessor with a fallback value when the key is not found.
/// Example: `theme_color_try!(color_bg, "bg.base", Color::Reset);`
macro_rules! theme_color_try {
    ($fn_name:ident, $key:literal, $fallback:expr) => {
        pub fn $fn_name() -> Color {
            crate::theme::current_theme()
                .try_color($key)
                .map(Color::from)
                .unwrap_or($fallback)
        }
    };
}

// Generated color accessors
theme_color_try!(color_bg, "bg.base", Color::Reset);
theme_color_try!(color_bg_panel, "bg.panel", Color::Reset);
theme_color!(color_fg, "text.primary");
theme_color!(color_fg_mid, "text.secondary");
theme_color!(color_accent, "accent.primary");
theme_color!(color_success, "success");
theme_color!(color_warning, "warning");
theme_color!(color_error, "error");
theme_color!(color_dim, "text.dim");
theme_color!(color_border, "border.unfocused");
theme_color!(color_code, "code.function");
theme_color_try!(color_code_bg, "bg.code", Color::Reset);

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
/// Uses palette::Srgb for correct gamma-space darkening.
pub fn darken(color: Color, factor: f32) -> Color {
    match color {
        Color::Rgb(r, g, b) => {
            use palette::Srgb;
            // palette uses 0.0-1.0 range
            let s = Srgb::new(r as f32 / 255.0, g as f32 / 255.0, b as f32 / 255.0);
            // Darken by scaling toward black in display gamma space.
            let factor = factor.clamp(0.0, 1.0);
            Color::Rgb(
                (s.red * factor * 255.0) as u8,
                (s.green * factor * 255.0) as u8,
                (s.blue * factor * 255.0) as u8,
            )
        }
        _ => color,
    }
}

/// Blend two RGB colors with the given opacity (0.0-1.0).
/// Uses palette::Srgba for proper premultiplied-alpha blending.
fn blend(bg: Color, fg: Color, opacity: f32) -> Color {
    use palette::blend::BlendWith;
    use palette::blend::PreAlpha;
    use palette::Srgba;

    let opacity = opacity.clamp(0.0, 1.0);

    let (br, bb, bblue) = match bg {
        Color::Rgb(r, g, b) => (r as f32, g as f32, b as f32),
        _ => (30.0, 30.0, 30.0),
    };
    let (fr, fg_g, fb) = match fg {
        Color::Rgb(r, g, b) => (r as f32, g as f32, b as f32),
        _ => return fg,
    };

    // Convert to palette's 0.0-1.0 Srgba space.
    let bg_s: Srgba<f32> = Srgba::new(br / 255.0, bb / 255.0, bblue / 255.0, 1.0);
    let fg_s: Srgba<f32> = Srgba::new(fr / 255.0, fg_g / 255.0, fb / 255.0, opacity);

    // Standard over-compositing with premultiplied alpha.
    let bg_pre: PreAlpha<_> = bg_s.into();
    let fg_pre: PreAlpha<_> = fg_s.into();

    let out: PreAlpha<_> = fg_pre.blend_with(bg_pre, |src: PreAlpha<_>, dst: PreAlpha<_>| {
        // Standard over: dst * (1 - src_alpha) + src
        PreAlpha {
            color: src.color + dst.color * (1.0 - src.alpha),
            alpha: src.alpha + dst.alpha * (1.0 - src.alpha),
        }
    });

    // Convert back to Srgba, then to sRGB.
    let out: Srgba<f32> = out.into();
    Color::Rgb(
        (out.red.clamp(0.0, 1.0) * 255.0) as u8,
        (out.green.clamp(0.0, 1.0) * 255.0) as u8,
        (out.blue.clamp(0.0, 1.0) * 255.0) as u8,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn macro_generates_same_color_values() {
        // Verify macro-generated accessors return valid Color values.
        // These call the actual generated functions to ensure they compile and work.
        assert!(matches!(color_bg(), Color::Rgb(_, _, _) | Color::Reset));
        assert!(matches!(color_bg_panel(), Color::Rgb(_, _, _) | Color::Reset));
        assert!(matches!(color_fg(), Color::Rgb(_, _, _)));
        assert!(matches!(color_fg_mid(), Color::Rgb(_, _, _)));
        assert!(matches!(color_accent(), Color::Rgb(_, _, _)));
        assert!(matches!(color_success(), Color::Rgb(_, _, _)));
        assert!(matches!(color_warning(), Color::Rgb(_, _, _)));
        assert!(matches!(color_error(), Color::Rgb(_, _, _)));
        assert!(matches!(color_dim(), Color::Rgb(_, _, _)));
        assert!(matches!(color_border(), Color::Rgb(_, _, _)));
        assert!(matches!(color_code(), Color::Rgb(_, _, _)));
        assert!(matches!(color_code_bg(), Color::Rgb(_, _, _) | Color::Reset));
        assert!(matches!(color_fg_bright(), Color::Rgb(_, _, _)));
        assert!(matches!(color_diff_insert_bg(), Color::Rgb(_, _, _)));
        assert!(matches!(color_diff_remove_bg(), Color::Rgb(_, _, _)));
        assert!(matches!(color_user_bg(), Color::Rgb(_, _, _) | Color::Reset));
        assert!(matches!(color_accent_bg(), Color::Rgb(_, _, _)));
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
