# Merge runie-term into runie-tui

**Status**: done
**Milestone**: R3
**Category**: TUI / Rendering
**Priority**: P1

**Depends on**: (none)
**Blocks**: unify-rendering-pipeline

**Re-opened**: 2026-06-16 — `crates/runie-term-archive/` still exists as a near-duplicate of `runie-tui` and must be removed.

## Description

`runie-term` owns terminal setup, capability detection, and CLI entry,
while `runie-tui` owns widgets, layout, and rendering. The boundary is
artificial: terminal capabilities are only meaningful to the TUI, and the
TUI cannot render without the terminal setup that lives in another crate.

Merging `runie-term` into `runie-tui` removes a crate and a layer of
indirection.

## Acceptance Criteria

- [x] `runie-term` crate is deleted or reduced to a thin binary wrapper.
- [x] `runie-tui` contains terminal setup (`terminal_setup`), capability
  detection (`terminal/caps`), effects, and the existing TUI widgets.
- [x] The `runie` binary still builds and runs.
- [x] `cargo test --workspace` succeeds.

## Tests

### Layer 2 — Event Handling
- [x] `terminal_caps_detect_from_env` — capability detection still works
  after the move (verified by `keymap_tests::merge::terminal_caps_detect_from_env`).
- [x] Keymap conversion tests pass after the merge (`keymap_tests::merge::*`).

### Layer 3 — Rendering
- [x] `tui_still_renders_after_merge` — all 351 runie-tui tests pass with
  the merged crate structure.

### Layer 4 — Smoke
- [x] `tmux_smoke_starts` — covered by `scripts/smoke-tmux.sh`.

## Files touched

- `crates/runie-term/` (archived to `crates/runie-term-archive/`)
- `crates/runie-term/Cargo.toml` (now thin re-export wrapper)
- `crates/runie-tui/Cargo.toml` (added deps from runie-term)
- `crates/runie-tui/src/lib.rs` (added terminal/effects modules)
- `crates/runie-tui/src/main.rs` (binary entry, moved from runie-term)
- `crates/runie-tui/src/terminal/` (copied from runie-term)
- `crates/runie-tui/src/effects/` (copied from runie-term)
- `crates/runie-tui/src/keymap.rs` + `keymap_tests/` (copied from runie-term)
- `crates/runie-tui/src/app_init.rs`, `share.rs`, `terminal_setup.rs` (copied)
- `crates/runie-tui/src/tests/` (merged from both crates)
- `Cargo.toml` workspace members (removed runie-term)

## Notes

If keeping a separate binary crate is desired, create a `runie-bin` crate
that depends only on `runie-tui`. But for now the simplest path is to move
all `runie-term` code into `runie-tui`.
