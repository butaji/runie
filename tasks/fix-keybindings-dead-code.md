# Fix keybindings dead-code warning

**Status**: done
**Milestone**: R4
**Category**: Configuration
**Priority**: P3

**Depends on**: none
**Blocks**: cleanup-small-duplicates-and-dead-code

## Description

`crates/runie-core/src/keybindings/mod.rs:23` defines `parse_key_combo` with `#[allow(dead_code)]`, but the function is only used by tests. The allow is noisy and masks intent.

## Acceptance Criteria

- [ ] Convert `parse_key_combo` to `#[cfg(test)]` if it is test-only, or document it as `pub(crate)` with a justified `#[allow(dead_code)]` if it is meant for future use.
- [ ] `cargo check --workspace` succeeds with no new warnings.
- [ ] `cargo test --workspace` succeeds.

## Tests

### Layer 1 — State/Logic
- [ ] `parse_key_combo_still_tested` — existing keybinding tests still compile and pass.

## Files touched

- `crates/runie-core/src/keybindings/mod.rs`

## Notes

- Tiny independent cleanup. If converting to `#[cfg(test)]` moves the function out of production builds, make sure the existing `keybindings/tests.rs` still finds it.
