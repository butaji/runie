# Adopt `palette` for theme color math

**Status**: todo
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

- `opaline` is already used in `styles.rs`; evaluate whether `palette` is a better single dependency.
