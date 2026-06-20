use crate::semantic_tokens::DEFAULT_THEME_TOML;

/// List all available builtin theme names.
pub fn list_builtin_themes() -> Vec<&'static str> {
    runie_core::themes::BUILTIN_THEMES.to_vec()
}

pub(crate) fn default_theme() -> opaline::Theme {
    opaline::load_from_str(DEFAULT_THEME_TOML, None).expect("embedded default theme must be valid")
}

/// Load a theme by name: builtin → custom file → default fallback (no style registration).
pub(crate) fn load_theme_raw(name: &str) -> opaline::Theme {
    // Only use the builtin loader if the name is actually a builtin.
    // "runie" is not a builtin — it uses the embedded DEFAULT_THEME_TOML.
    if let Some(t) = opaline::load_by_name(name) {
        return t;
    }
    let custom_path = dirs::home_dir()
        .unwrap_or_else(|| std::path::PathBuf::from("."))
        .join(".runie")
        .join("themes")
        .join(format!("{}.toml", name));
    if let Ok(theme) =
        runie_core::async_io::block_in_place_if_runtime(|| opaline::load_from_file(&custom_path))
    {
        return theme;
    }
    default_theme()
}

/// Load a theme by name: builtin → custom file → default fallback.
pub(crate) fn load_theme(name: &str) -> opaline::Theme {
    crate::theme::styles::register_runie_styles(load_theme_raw(name))
}

/// Load a theme and quantize its colors to the terminal's color depth.
pub(crate) fn load_theme_with_caps(
    name: &str,
    caps: crate::terminal::caps::TerminalCapabilities,
) -> opaline::Theme {
    let base = load_theme(name);
    if caps.truecolor {
        return base; // No quantization needed
    }
    quantize_theme(base, caps, name)
}

/// Quantize all palette and token colors in a theme to the terminal's color depth.
fn quantize_theme(
    theme: opaline::Theme,
    caps: crate::terminal::caps::TerminalCapabilities,
    name: &str,
) -> opaline::Theme {
    use opaline::OpalineColor;

    // Determine target depth: ANSI16 if mouse is None (very limited terminal),
    // otherwise ANSI256.
    let depth = if caps.mouse == crate::terminal::caps::MouseCapability::None {
        crate::quantize::ColorDepth::ANSI16
    } else {
        crate::quantize::ColorDepth::ANSI256
    };

    // Collect quantized (name, OpalineColor) pairs from palette and tokens.
    let mut quantized: Vec<(String, OpalineColor)> = Vec::new();

    for name in theme.palette_names() {
        let c = theme.color(name);
        quantized.push((name.to_string(), quantize_opaline_color(c, depth)));
    }
    for name in theme.token_names() {
        let c = theme.color(name);
        quantized.push((name.to_string(), quantize_opaline_color(c, depth)));
    }

    // Reconstruct: load fresh theme and register quantized tokens on top.
    let mut result = load_theme_raw(name);
    for (k, v) in &quantized {
        result.register_token(k, *v);
    }
    crate::theme::styles::register_runie_styles(result)
}

/// Quantize an opaline color to the given depth, returning the nearest ANSI color.
fn quantize_opaline_color(
    c: opaline::OpalineColor,
    depth: crate::quantize::ColorDepth,
) -> opaline::OpalineColor {
    let rat = ratatui::style::Color::Rgb(c.r, c.g, c.b);
    let quantized = crate::quantize::quantize(rat, depth);
    match quantized {
        ratatui::style::Color::Indexed(i) => {
            // Map indexed color back to a reasonable RGB approximation.
            indexed_to_opaline(i)
        }
        ratatui::style::Color::Rgb(r, g, b) => opaline::OpalineColor::new(r, g, b),
        // Named/other colors pass through as fallback.
        _ => c,
    }
}

/// Approximate an ANSI color index as an OpalineColor (for quantized theme tokens).
fn indexed_to_opaline(i: u8) -> opaline::OpalineColor {
    // ANSI 16-color palette approximations.
    const ANSI16: [(u8, u8, u8); 16] = [
        (0x00, 0x00, 0x00), // 0  black
        (0xCD, 0x00, 0x00), // 1  red
        (0x00, 0xCD, 0x00), // 2  green
        (0xCD, 0xCD, 0x00), // 3  yellow
        (0x00, 0x00, 0xEE), // 4  blue
        (0xCD, 0x00, 0xCD), // 5  magenta
        (0x00, 0xCD, 0xCD), // 6  cyan
        (0xE5, 0xE5, 0xE5), // 7  white
        (0x7F, 0x7F, 0x7F), // 8  bright black
        (0xFF, 0x00, 0x00), // 9  bright red
        (0x00, 0xFF, 0x00), // 10 bright green
        (0xFF, 0xFF, 0x00), // 11 bright yellow
        (0x00, 0x00, 0xFF), // 12 bright blue
        (0xFF, 0x00, 0xFF), // 13 bright magenta
        (0x00, 0xFF, 0xFF), // 14 bright cyan
        (0xFF, 0xFF, 0xFF), // 15 bright white
    ];
    if (i as usize) < ANSI16.len() {
        let (r, g, b) = ANSI16[i as usize];
        opaline::OpalineColor::new(r, g, b)
    } else {
        opaline::OpalineColor::FALLBACK
    }
}
