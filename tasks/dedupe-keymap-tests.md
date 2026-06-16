# Dedupe Keymap Tests

**Status**: todo
**Milestone**: R3
**Category**: TUI / Rendering
**Priority**: P1

**Depends on**: (none)
**Blocks**: (none)

## Description

`crates/runie-tui/src/keymap_tests/` duplicates `crates/runie-tui/src/keymap/tests/` (`basic.rs`, `combos.rs`, `special.rs`, `mod.rs`). The same tests are compiled twice under different module paths.

## Acceptance Criteria

- [ ] Only one keymap test directory remains.
- [ ] `lib.rs` no longer declares both `mod keymap_tests;` and `keymap.rs`'s inline tests.
- [ ] All keymap tests still pass and count matches the unique set.

## Tests

### Layer 2 — Event Handling
- [ ] `keymap_tests_unique` — no duplicate test names compiled twice.
- [ ] `all_keymap_tests_pass` — existing assertions still hold.

## Files touched

- `crates/runie-tui/src/keymap_tests/` (deleted)
- `crates/runie-tui/src/lib.rs`

## Notes

Prefer keeping `src/keymap/tests/` because it is the idiomatic Rust location.
