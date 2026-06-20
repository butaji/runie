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
    if let Ok(theme) = opaline::load_from_file(&custom_path) {
        return theme;
    }
    default_theme()
}

/// Async variant of `load_theme_raw` — offloads file I/O from the async runtime.
pub(crate) async fn load_theme_raw_async(name: String) -> opaline::Theme {
    tokio::task::spawn_blocking(move || load_theme_raw(&name))
        .await
        .unwrap_or_else(|_| default_theme())
}

/// Load a theme by name: builtin → custom file → default fallback.
pub(crate) fn load_theme(name: &str) -> opaline::Theme {
    crate::theme::styles::register_runie_styles(load_theme_raw(name))
}

/// Async variant of `load_theme`.
pub(crate) async fn load_theme_async(name: String) -> opaline::Theme {
    let raw = load_theme_raw_async(name).await;
    crate::theme::styles::register_runie_styles(raw)
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

/// Async variant of `load_theme_with_caps`.
pub(crate) async fn load_theme_with_caps_async(
    name: String,
    caps: crate::terminal::caps::TerminalCapabilities,
) -> opaline::Theme {
    let base = load_theme_async(name.clone()).await;
    if caps.truecolor {
        return base;
    }
    quantize_theme(base, caps, &name)
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
    if i < 16 {
        return ansi16_to_opaline(i);
    }
    if i < 232 {
        return ansi256_cube_to_opaline(i);
    }
    ansi256_gray_to_opaline(i)
}

fn ansi16_to_opaline(i: u8) -> opaline::OpalineColor {
    const ANSI16: [(u8, u8, u8); 16] = [
        (0x00, 0x00, 0x00), (0xCD, 0x00, 0x00), (0x00, 0xCD, 0x00),
        (0xCD, 0xCD, 0x00), (0x00, 0x00, 0xEE), (0xCD, 0x00, 0xCD),
        (0x00, 0xCD, 0xCD), (0xE5, 0xE5, 0xE5), (0x7F, 0x7F, 0x7F),
        (0xFF, 0x00, 0x00), (0x00, 0xFF, 0x00), (0xFF, 0xFF, 0x00),
        (0x00, 0x00, 0xFF), (0xFF, 0x00, 0xFF), (0x00, 0xFF, 0xFF),
        (0xFF, 0xFF, 0xFF),
    ];
    let (r, g, b) = ANSI16[i as usize];
    opaline::OpalineColor::new(r, g, b)
}

fn ansi256_cube_to_opaline(i: u8) -> opaline::OpalineColor {
    let n = i - 16;
    let r = (n / 36) as u8;
    let g = ((n % 36) / 6) as u8;
    let b = (n % 6) as u8;
    let channel = |v: u8| if v == 0 { 0 } else { 95 + (v - 1) * 40 };
    opaline::OpalineColor::new(channel(r), channel(g), channel(b))
}

fn ansi256_gray_to_opaline(i: u8) -> opaline::OpalineColor {
    let gray = 8 + (i - 232) * 10;
    opaline::OpalineColor::new(gray, gray, gray)
}
