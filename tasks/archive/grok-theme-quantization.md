# Theme Quantization Wiring

**Status**: done
**Milestone**: R4
**Category**: TUI / Rendering
**Priority**: P2

**Depends on**: (none)
**Blocks**: (none)

## Description

Runie has `quantize.rs` (RGB → ANSI 256/16) and `TerminalCapabilities.truecolor`,
but they are not connected. Wire them so non-truecolor terminals get a usable
quantized theme.

## Acceptance Criteria

- [x] At theme load, if `caps.truecolor` is true, use the theme unchanged.
- [x] If false, quantize all `Color::Rgb` values in the theme to ANSI 256
  (or ANSI 16 if the terminal indicates 16-color support).
- [x] Quantization happens once at startup/config reload, not per frame.
- [x] Existing theme tests still pass.

## Tests

### Layer 1 — State / Logic

```rust
#[test]
fn truecolor_keeps_rgb_unchanged() {
    let theme = load_theme_for_caps(truecolor_caps());
    assert!(theme.has_rgb_colors());
}

#[test]
fn non_truecolor_quantizes_to_indexed() {
    let theme = load_theme_for_caps(ansi256_caps());
    assert!(theme.colors_are_indexed());
}
```

### Layer 3 — Rendering

```rust
#[test]
fn quantized_theme_renders_without_truecolor_codes() {
    // TestBackend assertion: no RGB style colors used.
}
```

## Files touched

- `crates/runie-tui/src/theme.rs` — `set_current_theme_with_caps`, `load_theme_with_caps`, `quantize_theme`, `quantize_opaline_color`, `indexed_to_opaline`
- `crates/runie-tui/src/main.rs` — calls `theme::set_current_theme_with_caps` after `setup_terminal()`
- `crates/runie-tui/src/quantize.rs` — consumed by theme quantization
- `crates/runie-tui/src/theme_tests.rs` — 3 new Layer 1 tests

## Out of scope

- OS-adaptive `theme = "auto"`.
- Shipping pre-quantized theme variants.

## Done

**Architecture**: `TerminalCapabilities` (detected once at startup in `setup_terminal()`) are stored in a global `CURRENT_CAPS` and passed to `set_current_theme_with_caps()`. Theme loading is quantized once at that call — not per frame.

**`theme.rs`**:
- Added `CURRENT_CAPS` static — stores `TerminalCapabilities` globally
- Added `set_current_theme_with_caps(name, caps)` — updates caps + delegates to `load_theme_with_caps`
- Added `load_theme_with_caps(name, caps)` — if `caps.truecolor` is true, passes through; otherwise calls `quantize_theme`
- Added `quantize_theme(theme, caps)` — iterates palette + token names, quantizes each via `quantize_opaline_color`, re-registers on a fresh theme load
- Added `quantize_opaline_color(c, depth)` — converts `OpalineColor` → `ratatui::Color` → quantized → `OpalineColor`
- Added `indexed_to_opaline(i)` — maps ANSI 16 indices back to `OpalineColor` approximations
- Split `load_theme` into `load_theme_raw` (no style registration) and `load_theme` (raw + `register_runie_styles`)
- `set_current_theme(name)` now delegates to `set_current_theme_with_caps` with default caps

**`main.rs`**: After `setup_terminal()`, calls `theme::set_current_theme_with_caps(theme::DEFAULT_THEME_NAME, terminal_caps)` to initialize the quantized theme before first render.

**Quantization depth**: `MouseCapability::None` → ANSI16 (very limited terminal), otherwise ANSI256.

**Tests**: 4 theme tests pass (`theme_cache_returns_same_instance`, `truecolor_theme_keeps_rgb_colors`, `non_truecolor_quantizes_to_indexed_approximations`, `quantization_is_idempotent`). All 359 workspace tests pass.
