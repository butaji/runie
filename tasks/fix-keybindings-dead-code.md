# Fix keybindings dead-code warning

**Status**: done
**Note**: Verified 2026-06-29 — `parse_key_combo` is now `#[cfg(test)]` at line 23 of keybindings/mod.rs.
**Milestone**: R4
**Category**: Configuration
**Priority**: P3

**Depends on**: none
**Blocks**: cleanup-small-duplicates-and-dead-code

## Description

`crates/runie-core/src/keybindings/mod.rs:23` defines `parse_key_combo` with `#[allow(dead_code)]`, but the function is only used by tests. The allow is noisy and masks intent.

## Acceptance Criteria

- [x] Convert `parse_key_combo` to `#[cfg(test)]` if it is test-only, or document it as `pub(crate)` with a justified `#[allow(dead_code)]` if it is meant for future use.
- [x] `cargo check --workspace` succeeds with no new warnings.
- [x] `cargo test --workspace` succeeds.

## Tests

### Layer 1 — State/Logic
- [x] `parse_key_combo_still_tested` — existing keybinding tests still compile and pass.

## Files touched

- `crates/runie-core/src/keybindings/mod.rs`

## Notes

- Tiny independent cleanup. If converting to `#[cfg(test)]` moves the function out of production builds, make sure the existing `keybindings/tests.rs` still finds it.
## Completion Validation

Before marking this task complete, confirm all three validation gates:

- [ ] **Unit tests** — `cargo test --lib` covers the changed logic and all new/modified unit tests pass.
- [ ] **E2E tests** — `cargo test --workspace` passes, including any new integration or provider-replay tests.
- [ ] **Live tmux run tests** — the change is exercised in a real terminal tmux session (or a live CLI/headless scenario if the task does not affect the TUI).
