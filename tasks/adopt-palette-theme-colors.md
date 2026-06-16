# Adopt `palette` for Theme Color Helpers

**Status**: done
**Milestone**: R3
**Category**: TUI / Rendering
**Priority**: P2

**Depends on**: (none)
**Blocks**: (none)

## Description

Replace the hand-rolled RGB `darken` and alpha-blend helpers in `crates/runie-tui/src/theme.rs` with the `palette` crate. `palette` provides color-space-correct color manipulation and easy conversion to `ratatui::style::Color`.

## Acceptance Criteria

- [x] `palette` is added as a dependency to `runie-tui`.
- [x] `theme.rs` helpers use `palette` for darken/lighten/blend operations.
- [x] Output is visually equivalent or improved; add render tests for any shifts.
- [x] `cargo test --workspace` succeeds.

## Tests

### Layer 1 — State/Logic
- [x] `palette_darken_uses_palette_types` — `darken` works with palette's Srgb.
- [x] `palette_blend_uses_palette_types` — `blend` works with palette's Srgba+PreAlpha.
- [x] `palette_blend_with_zero_opacity_returns_bg` — opacity=0 returns background unchanged.

### Layer 2 — Event Handling
N/A.

### Layer 3 — Rendering
- [x] All 379 runie-tui tests pass (existing theme/render tests).

### Layer 4 — Smoke / Crash
N/A.

## Files touched

- `Cargo.toml` (workspace dep)
- `crates/runie-tui/Cargo.toml` (local dep)
- `crates/runie-tui/src/theme.rs`

## Implementation Notes

- `palette = "0.7"` added as workspace dependency.
- `darken()` uses `Srgb` with `mix(black, factor)` via `palette::Blend` trait.
- `blend()` uses `Srgba<f32>` + `PreAlpha` with standard over-compositing via `palette::blend::BlendWith`.
- The `mix` method lives on `PreAlpha`, not plain `Srgb`; used `blend_with` with a custom closure instead.
