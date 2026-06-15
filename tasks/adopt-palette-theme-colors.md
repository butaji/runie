# Adopt `palette` for Theme Color Helpers

**Status**: todo
**Milestone**: R3
**Category**: TUI / Rendering
**Priority**: P2

**Depends on**: (none)
**Blocks**: (none)

## Description

Replace the hand-rolled RGB `darken` and alpha-blend helpers in `crates/runie-tui/src/theme.rs` with the `palette` crate. `palette` provides color-space-correct color manipulation and easy conversion to `ratatui::style::Color`.

## Acceptance Criteria

- [ ] `palette` is added as a dependency to `runie-tui`.
- [ ] `theme.rs` helpers use `palette` for darken/lighten/blend operations.
- [ ] Output is visually equivalent or improved; add render tests for any shifts.
- [ ] `cargo test --workspace` succeeds.

## Tests

### Layer 1 — State/Logic
- [ ] `palette_darken_matches_legacy` — `palette` darken produces the same or better result as the legacy helper.

### Layer 2 — Event Handling
N/A.

### Layer 3 — Rendering
- [ ] `theme_colors_render_consistently` — TUI theme colors render as expected.

### Layer 4 — Smoke / Crash
N/A.

## Files touched

- `crates/runie-tui/Cargo.toml`
- `crates/runie-tui/src/theme.rs`

## Notes

- Ratatui has an optional `palette` feature for easier interop; check if it simplifies conversion.
- See `docs/CRATE_DECISIONS.md`.
