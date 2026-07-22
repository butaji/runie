use crate::semantic_tokens::DEFAULT_THEME_TOML;

/// Minimal fallback theme TOML used when the embedded default theme fails to load.
/// This is a hardcoded constant that cannot fail to parse — used only in the
/// last-resort fallback path in `current_theme()`.
const MINIMAL_FALLBACK_TOML: &str = concat!(
    "[meta]\n",
    "name = \"runie-minimal\"\n",
    "author = \"runie\"\n",
    "variant = \"dark\"\n",
    "\n",
    "[palette]\n",
    "bg-base = \"#1e1e1e\"\n",
    "text-primary = \"#cccccc\"\n",
    "accent-primary = \"#569cd6\"\n",
    "success = \"#4ec9b0\"\n",
    "error = \"#f14c4c\"\n",
    "\n",
    "[tokens]\n",
    "bg-base = \"#1e1e1e\"\n",
    "text-primary = \"#cccccc\"\n",
    "accent-primary = \"#569cd6\"\n",
    "success = \"#4ec9b0\"\n",
    "error = \"#f14c4c\"\n",
);

/// Last-resort fallback theme. Used only when ALL loaders (builtin, custom file,
/// and embedded default) fail — which would indicate build-pipeline corruption.
/// This constant TOML is designed to be trivially parseable and never fails.
pub(crate) fn minimal_fallback_theme() -> opaline::Theme {
    // This is a hardcoded TOML constant designed to be trivially parseable.
    // Panic only if the opaline API has changed incompatibly.
    opaline::load_from_str(MINIMAL_FALLBACK_TOML, None).unwrap()
}

// Canonical source for built-in theme names; also used by runie-core for the CLI.
pub use runie_core::theme_tokens::BUILTIN_THEMES;

/// List all available builtin theme names.
pub fn list_builtin_themes() -> Vec<&'static str> {
    BUILTIN_THEMES.to_vec()
}

/// Load the embedded default theme.
///
/// Returns an error only if the embedded TOML is syntactically invalid
/// (which would indicate build-pipeline corruption).
pub(crate) fn default_theme() -> Result<opaline::Theme, opaline::OpalineError> {
    opaline::load_from_str(DEFAULT_THEME_TOML, None)
}

/// Load a theme by name: builtin → custom file → default fallback (no style registration).
pub(crate) fn load_theme_raw(name: &str) -> Result<opaline::Theme, opaline::OpalineError> {
    // Only use the builtin loader if the name is actually a builtin.
    // "runie" is not a builtin — it uses the embedded DEFAULT_THEME_TOML.
    if let Some(t) = opaline::load_by_name(name) {
        return Ok(t);
    }
    let custom_path = dirs::home_dir()
        .unwrap_or_else(|| std::path::PathBuf::from("."))
        .join(".runie")
        .join("themes")
        .join(format!("{}.toml", name));
    if let Ok(theme) = opaline::load_from_file(&custom_path) {
        return Ok(theme);
    }
    default_theme()
}

/// Load a theme by name: builtin → custom file → default fallback.
pub(crate) fn load_theme(name: &str) -> Result<opaline::Theme, opaline::OpalineError> {
    load_theme_raw(name).map(crate::theme::styles::register_runie_styles)
}

/// Load a theme and quantize its colors to the terminal's color depth.
pub(crate) fn load_theme_with_caps(
    name: &str,
    caps: crate::terminal::caps::TermCaps,
) -> Result<opaline::Theme, opaline::OpalineError> {
    let base = load_theme(name)?;
    if caps.truecolor {
        return Ok(base); // No quantization needed
    }
    quantize_theme(base, caps, name)
}

/// Quantize all palette and token colors in a theme to the terminal's color depth.
fn quantize_theme(
    theme: opaline::Theme,
    caps: crate::terminal::caps::TermCaps,
    name: &str,
) -> Result<opaline::Theme, opaline::OpalineError> {
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
        quantized.push((name.to_owned(), quantize_opaline_color(c, depth)));
    }
    for name in theme.token_names() {
        let c = theme.color(name);
        quantized.push((name.to_owned(), quantize_opaline_color(c, depth)));
    }

    // Reconstruct: load fresh theme and register quantized tokens on top.
    let mut result = load_theme_raw(name)?;
    for (k, v) in &quantized {
        result.register_token(k, *v);
    }
    Ok(crate::theme::styles::register_runie_styles(result))
}

/// Quantize an opaline color to the given depth, returning the nearest ANSI color.
fn quantize_opaline_color(c: opaline::OpalineColor, depth: crate::quantize::ColorDepth) -> opaline::OpalineColor {
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
/// Delegates to `ansi_colours::rgb_from_ansi256` which handles all three ranges
/// (ANSI16, ANSI256 cube, ANSI256 gray) with the canonical xterm-256 formulas.
fn indexed_to_opaline(i: u8) -> opaline::OpalineColor {
    let (r, g, b) = ansi_colours::rgb_from_ansi256(i);
    opaline::OpalineColor::new(r, g, b)
}
