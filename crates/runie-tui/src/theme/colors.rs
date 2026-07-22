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
pub fn color_bg_user() -> Color {
    Color::from(crate::theme::styles::bg_user_color(
        &crate::theme::current_theme(),
    ))
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

/// User message text color (grok parity: neutral light gray).
/// Themes without `feed.user.fg` keep the legacy bright primary on dark
/// themes; light themes use the plain primary so text keeps full contrast
/// on the light card band.
pub fn color_user_text() -> Color {
    theme_color_fallback("feed.user.fg", default_user_text_color())
}

/// User text fallback: brightened primary on dark themes, plain primary on
/// light themes (brightening would wash out contrast on a light card).
fn default_user_text_color() -> Color {
    let theme = crate::theme::current_theme();
    let primary = theme.color("text.primary");
    let is_light = theme
        .try_color("bg.base")
        .map(|b| 0.299 * f32::from(b.r) + 0.587 * f32::from(b.g) + 0.114 * f32::from(b.b) > 128.0)
        .unwrap_or(false);
    let c = if is_light {
        primary
    } else {
        primary.lighten(0.3)
    };
    Color::Rgb(c.r, c.g, c.b)
}

/// Assistant answer text color (grok parity: neutral gray).
/// Themes without `feed.agent.fg` keep the primary text color.
pub fn color_agent_text() -> Color {
    theme_color_fallback("feed.agent.fg", color_fg())
}

/// Subagent running state color (grok parity: purple accent).
pub fn color_subagent_running() -> Color {
    theme_color_fallback("subagent.running", Color::Rgb(180, 90, 240))
}

/// Subagent completed state color (grok parity: success green).
pub fn color_subagent_completed() -> Color {
    theme_color_fallback("subagent.completed", color_success())
}

/// Subagent failed state color (grok parity: error red).
pub fn color_subagent_failed() -> Color {
    theme_color_fallback("subagent.failed", color_error())
}

// Grok-style feed sub-agent lifecycle row accents (GROK.md §26).
// These are intentionally granular so the diamond, left bar, and body can be
// colored independently while still being themeable.

/// Running left bar `❙` color (grok: `#685786`).
pub fn color_subagent_running_bar() -> Color {
    theme_color_fallback("subagent.running.bar", Color::Rgb(104, 87, 134))
}

/// Running diamond `◆` color (grok: `#332d3e`).
pub fn color_subagent_running_diamond() -> Color {
    theme_color_fallback("subagent.running.diamond", Color::Rgb(51, 45, 62))
}

/// Space/dim accent between running diamond and text (grok: `#685786`).
pub fn color_subagent_running_dim() -> Color {
    theme_color_fallback("subagent.running.dim", Color::Rgb(104, 87, 134))
}

/// Completed diamond `◆` color (grok: `#59713f`).
pub fn color_subagent_completed_diamond() -> Color {
    theme_color_fallback("subagent.completed.diamond", Color::Rgb(89, 113, 63))
}

/// Bright accent after completed diamond (grok: `#9ece6a`).
pub fn color_subagent_completed_bright() -> Color {
    theme_color_fallback("subagent.completed.bright", Color::Rgb(158, 206, 106))
}

/// Failed diamond `◆` color (grok: `#864551`).
pub fn color_subagent_failed_diamond() -> Color {
    theme_color_fallback("subagent.failed.diamond", Color::Rgb(134, 69, 81))
}

/// Bright accent after failed diamond (grok: `#f7768e`).
pub fn color_subagent_failed_bright() -> Color {
    theme_color_fallback("subagent.failed.bright", Color::Rgb(247, 118, 142))
}

// Grok-style semantic accent colors (grok-build parity).

/// Thinking/plan/feedback accent: purple (grok: `#9D7CD8`).
pub fn color_thinking() -> Color {
    theme_color_fallback("accent.thinking", Color::Rgb(157, 124, 216))
}

/// Plan approval accent: gold (grok: `#E0AF68`).
pub fn color_plan() -> Color {
    theme_color_fallback("accent.plan", Color::Rgb(224, 175, 104))
}

/// Feedback/user-response accent: teal (grok: `#73DACA`).
pub fn color_feedback() -> Color {
    theme_color_fallback("accent.feedback", Color::Rgb(115, 218, 202))
}

/// Monitor/watch pulse accent: amber (grok: `#FF9E64`).
pub fn color_monitor() -> Color {
    theme_color_fallback("accent.monitor", Color::Rgb(255, 158, 100))
}

// Grok-style multi-tier block backgrounds (grok-build parity).

/// Light block background (grok: `#2E2E2E`).
pub fn color_bg_light() -> Color {
    theme_color_fallback("bg.light", Color::Rgb(46, 46, 46))
}

/// Dark block background (grok: `#101010`).
pub fn color_bg_dark() -> Color {
    theme_color_fallback("bg.dark", Color::Rgb(16, 16, 16))
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
// Grok-style indexed color support (grok-build parity)
// ─────────────────────────────────────────────────────────────────────────────

/// The 6 channel values in the 256-color 6×6×6 cube.
const CUBE_VALUES: [u8; 6] = [0, 95, 135, 175, 215, 255];

/// Convert a 256-color indexed color to its (R, G, B) components.
///
/// Handles all three regions of the 256-color palette:
/// - 0–15:    standard/bright ANSI colors (uses common xterm defaults)
/// - 16–231:  6×6×6 color cube
/// - 232–255: 24-step grayscale ramp
pub fn indexed_to_rgb(index: u8) -> (u8, u8, u8) {
    match index {
        // Standard colors (0–7) — common xterm defaults
        0 => (0, 0, 0),
        1 => (128, 0, 0),
        2 => (0, 128, 0),
        3 => (128, 128, 0),
        4 => (0, 0, 128),
        5 => (128, 0, 128),
        6 => (0, 128, 128),
        7 => (192, 192, 192),
        // Bright colors (8–15)
        8 => (128, 128, 128),
        9 => (255, 0, 0),
        10 => (0, 255, 0),
        11 => (255, 255, 0),
        12 => (0, 0, 255),
        13 => (255, 0, 255),
        14 => (0, 255, 255),
        15 => (255, 255, 255),
        // 6×6×6 color cube (16–231)
        16..=231 => {
            let n = index - 16;
            let r = CUBE_VALUES[(n / 36) as usize];
            let g = CUBE_VALUES[((n % 36) / 6) as usize];
            let b = CUBE_VALUES[(n % 6) as usize];
            (r, g, b)
        }
        // Grayscale ramp (232–255): value = 8 + (index − 232) × 10
        232..=255 => {
            let v = 8 + (index - 232) * 10;
            (v, v, v)
        }
    }
}

/// Map an RGB triplet to the nearest 256-color palette index (16–255).
fn nearest_cube_channel(v: u8) -> u8 {
    let mut best = 0u8;
    let mut best_d = v.abs_diff(CUBE_VALUES[0]) as u16;
    for i in 1..6u8 {
        let d = v.abs_diff(CUBE_VALUES[i as usize]) as u16;
        if d < best_d {
            best = i;
            best_d = d;
        }
    }
    best
}

/// Squared Euclidean distance between two RGB colors.
fn sq_dist(r1: u8, g1: u8, b1: u8, r2: u8, g2: u8, b2: u8) -> u32 {
    let dr = r1 as i32 - r2 as i32;
    let dg = g1 as i32 - g2 as i32;
    let db = b1 as i32 - b2 as i32;
    (dr * dr + dg * dg + db * db) as u32
}

/// Map an RGB triplet to the nearest 256-color palette index (16–255).
pub fn nearest_indexed(r: u8, g: u8, b: u8) -> u8 {
    // Nearest in the 6×6×6 color cube (16–231)
    let ri = nearest_cube_channel(r);
    let gi = nearest_cube_channel(g);
    let bi = nearest_cube_channel(b);
    let cube_idx = 16 + 36 * ri as u16 + 6 * gi as u16 + bi as u16;
    let cube_dist = sq_dist(
        r,
        g,
        b,
        CUBE_VALUES[ri as usize],
        CUBE_VALUES[gi as usize],
        CUBE_VALUES[bi as usize],
    );

    // Nearest in the grayscale ramp (232–255)
    let lum = (r as u16 + g as u16 + b as u16) / 3;
    let gray_step = if lum <= 3 {
        0u8
    } else if lum >= 243 {
        23
    } else {
        ((lum as i16 - 8 + 5) / 10).clamp(0, 23) as u8
    };
    let gv = (8 + gray_step as u16 * 10) as u8;
    let gray_dist = sq_dist(r, g, b, gv, gv, gv);

    if gray_dist < cube_dist {
        232 + gray_step
    } else {
        cube_idx as u8
    }
}

/// Blend a single color channel: lerp from base toward original based on opacity.
///
/// - `opacity = 0.0`: returns `base` (fully faded)
/// - `opacity = 1.0`: returns `original` (no change)
#[inline]
pub fn blend_channel(base: u8, original: u8, opacity: f32) -> u8 {
    let result = base as f32 * (1.0 - opacity) + original as f32 * opacity;
    result.round() as u8
}

/// Extract (R, G, B) from a Color, supporting both Rgb and Indexed variants.
fn color_to_rgb(color: Color) -> Option<(u8, u8, u8)> {
    match color {
        Color::Rgb(r, g, b) => Some((r, g, b)),
        Color::Indexed(n) => Some(indexed_to_rgb(n)),
        _ => None,
    }
}

/// Blend a color toward a base color based on opacity.
///
/// - `opacity = 0.0`: returns `base` (fully faded)
/// - `opacity = 1.0`: returns `original` (no change)
///
/// Supports both `Color::Rgb` and `Color::Indexed` colors (indexed colors are
/// converted to their RGB equivalents for blending). When either input is
/// `Color::Indexed`, the blended result is quantized back to the nearest
/// 256-color index so the output stays terminal-compatible.
pub fn blend_color(base: Color, original: Color, opacity: f32) -> Option<Color> {
    let (base_r, base_g, base_b) = color_to_rgb(base)?;
    let (orig_r, orig_g, orig_b) = color_to_rgb(original)?;

    let r = blend_channel(base_r, orig_r, opacity);
    let g = blend_channel(base_g, orig_g, opacity);
    let b = blend_channel(base_b, orig_b, opacity);

    // When either input is indexed, quantize the blended result back to the
    // nearest 256-color index so the output stays terminal-compatible.
    Some(match (base, original) {
        (Color::Indexed(_), _) | (_, Color::Indexed(_)) => Color::Indexed(nearest_indexed(r, g, b)),
        _ => Color::Rgb(r, g, b),
    })
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
        assert_rgb(color_user_text());
        assert_rgb(color_agent_text());
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
