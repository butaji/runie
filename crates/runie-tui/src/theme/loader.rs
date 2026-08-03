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
    let name = resolve_theme_name(name);
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

/// Resolve Grok-compatible `auto`/`system` aliases without adding a second
/// theme-loading path. Explicit themes always win; desktop hints are only
/// consulted for the aliases.
fn resolve_theme_name(name: &str) -> &str {
    if !matches!(name.to_ascii_lowercase().as_str(), "auto" | "system") {
        return name;
    }
    let light = std::env::var("RUNIE_THEME_APPEARANCE")
        .or_else(|_| std::env::var("COLORFGBG"))
        .map(|value| {
            let value = value.to_ascii_lowercase();
            value.contains("light") || value.ends_with(";15") || value.ends_with(";default")
        })
        .unwrap_or(false);
    if light {
        "catppuccin-latte"
    } else {
        "runie"
    }
}

/// Load a theme by name: builtin → custom file → default fallback.
pub(crate) fn load_theme(name: &str) -> Result<opaline::Theme, opaline::OpalineError> {
    load_theme_raw(name)
        .map(ensure_runie_tokens)
        .map(|theme| {
            debug_assert!(
                validate_theme(&theme).is_ok(),
                "loaded theme violates Runie semantic contract"
            );
            theme
        })
        .map(crate::theme::styles::register_runie_styles)
}

/// Add Runie-specific semantic roles to every theme without changing roles
/// that the theme already defines. Built-in palettes therefore keep their own
/// hue family while the feed can rely on one stable token vocabulary.
pub(crate) fn ensure_runie_tokens(mut theme: opaline::Theme) -> opaline::Theme {
    let derived = [
        ("accent.thinking", "accent.primary"),
        ("accent.plan", "warning"),
        ("accent.feedback", "text.secondary"),
        ("accent.monitor", "accent.primary"),
        ("rail.running", "accent.primary"),
        ("rail.success", "success"),
        ("rail.error", "error"),
        ("rail.thinking", "text.dim"),
    ];
    for (token, source) in derived {
        if theme.try_color(token).is_none() {
            let color = theme.color(source);
            theme.register_token(token, color);
        }
    }
    theme
}

/// Validate the semantic contract consumed by the renderer. This is kept
/// separate from loading so diagnostics and theme tests can report the exact
/// missing role instead of silently displaying opaline's fallback color.
pub(crate) fn validate_theme(theme: &opaline::Theme) -> Result<(), String> {
    const REQUIRED: &[&str] = &[
        "bg.base",
        "bg.selection",
        "text.primary",
        "text.dim",
        "border.unfocused",
        "border.focused",
        "accent.primary",
        "success",
        "warning",
        "error",
        "rail.running",
        "rail.success",
        "rail.error",
        "rail.thinking",
    ];
    for token in REQUIRED {
        if theme.try_color(token).is_none() {
            return Err(format!("missing semantic token {token}"));
        }
    }
    let bg = theme.color("bg.base");
    let fg = theme.color("text.primary");
    if relative_luminance_contrast(bg, fg) < 2.0 {
        return Err("text.primary has insufficient contrast against bg.base".into());
    }
    let dim = theme.color("text.dim");
    if relative_luminance_contrast(bg, dim) < 1.2 {
        return Err("text.dim has insufficient contrast against bg.base".into());
    }
    let selection = theme.color("bg.selection");
    if relative_luminance_contrast(bg, selection) < 1.05 {
        return Err("bg.selection is indistinguishable from bg.base".into());
    }
    Ok(())
}

fn relative_luminance_contrast(a: opaline::OpalineColor, b: opaline::OpalineColor) -> f32 {
    fn lum(c: opaline::OpalineColor) -> f32 {
        let channel = |v: u8| {
            let v = f32::from(v) / 255.0;
            if v <= 0.03928 {
                v / 12.92
            } else {
                ((v + 0.055) / 1.055).powf(2.4)
            }
        };
        0.2126 * channel(c.r) + 0.7152 * channel(c.g) + 0.0722 * channel(c.b)
    }
    let (high, low) = (lum(a).max(lum(b)), lum(a).min(lum(b)));
    (high + 0.05) / (low + 0.05)
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
    let mut result = ensure_runie_tokens(load_theme_raw(name)?);
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
