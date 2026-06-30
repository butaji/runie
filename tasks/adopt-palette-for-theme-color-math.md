# Adopt `palette` for theme color math

**Status**: wontfix
**Milestone**: R7
**Category**: TUI / Rendering
**Priority**: P2

**Depends on**: replace-custom-tui-widgets-with-ratatui-ecosystem
**Blocks**: none

## Description

`crates/runie-tui/src/theme/colors.rs` implements hand-rolled `darken` and `blend` over sRGB. Replace them with the `palette` crate, which is the standard choice in the ratatui ecosystem.

## Acceptance Criteria

- [ ] Replace `darken`/`blend` with `palette` equivalents.
- [ ] Preserve existing theme color outputs (or document intentional changes).
- [ ] `cargo check --workspace` and `cargo test --workspace` pass.

## Tests

### Layer 1 — State/Logic
- [ ] `palette_darken_matches_legacy` — legacy and palette outputs match within tolerance.

## Files touched

- `crates/runie-tui/src/theme/colors.rs`
- `crates/runie-tui/Cargo.toml`

## Notes

**Won't fix**: The hand-rolled `darken` and `blend` functions are simple, correct, and well-tested. They implement standard sRGB linear interpolation which is appropriate for TUI contexts. The current implementation:

1. **Is correct**: Uses proper linear interpolation over sRGB components
2. **Is simple**: ~50 lines of straightforward Rust code
3. **Is tested**: Multiple unit tests verify correctness
4. **Has no dependencies**: Doesn't require adding `palette` crate

`palette` would be valuable for more advanced color space conversions (LAB, LCH, etc.) or perceptual color math, but the current use case is simple sRGB blending. Adding `palette` would increase compile times and binary size without clear benefit.
