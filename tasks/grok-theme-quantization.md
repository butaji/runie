# Theme Quantization Wiring

**Status**: todo
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

- [ ] At theme load, if `caps.truecolor` is true, use the theme unchanged.
- [ ] If false, quantize all `Color::Rgb` values in the theme to ANSI 256
  (or ANSI 16 if the terminal indicates 16-color support).
- [ ] Quantization happens once at startup/config reload, not per frame.
- [ ] Existing theme tests still pass.

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

- `crates/runie-tui/src/theme.rs`
- `crates/runie-term/src/terminal_setup.rs` (pass caps to theme load)
- `crates/runie-tui/src/quantize.rs`

## Out of scope

- OS-adaptive `theme = "auto"`.
- Shipping pre-quantized theme variants.
