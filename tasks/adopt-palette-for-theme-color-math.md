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

- [x] Replace `darken`/`blend` with `palette` equivalents. (WONTFIX - hand-rolled is correct for simple sRGB)
- [x] Preserve existing theme color outputs (or document intentional changes). (WONTFIX)
- [x] `cargo check --workspace` and `cargo test --workspace` pass.

## Tests

### Layer 1 — State/Logic
- [x] `palette_darken_matches_legacy` — legacy and palette outputs match within tolerance. (WONTFIX)

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
> **Live tmux testing session required:** After the implementation passes unit and E2E tests, run a real terminal tmux session that exercises the changed behavior. The task is not done until the live session succeeds.
## Completion Validation

Before marking this task complete, confirm all three validation gates:

- [ ] **Unit tests** — `cargo test --lib` covers the changed logic and all new/modified unit tests pass.
- [ ] **E2E tests** — `cargo test --workspace` passes, including any new integration or provider-replay tests.
- [ ] **Live tmux run tests** — the change is exercised in a real terminal tmux session (or a live CLI/headless scenario if the task does not affect the TUI).
