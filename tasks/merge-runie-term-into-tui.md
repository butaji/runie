# Merge runie-term into runie-tui

**Status**: todo
**Milestone**: R3
**Category**: TUI / Rendering
**Priority**: P1

**Depends on**: (none)
**Blocks**: `unify-rendering-pipeline`

## Description

`runie-term` owns terminal setup, capability detection, and CLI entry,
while `runie-tui` owns widgets, layout, and rendering. The boundary is
artificial: terminal capabilities are only meaningful to the TUI, and the
TUI cannot render without the terminal setup that lives in another crate.

Merging `runie-term` into `runie-tui` removes a crate and a layer of
indirection.

## Acceptance Criteria

- [ ] `runie-term` crate is deleted or reduced to a thin binary wrapper.
- [ ] `runie-tui` contains terminal setup (`terminal_setup`), capability
  detection (`terminal/caps`), effects, and the existing TUI widgets.
- [ ] The `runie` binary still builds and runs.
- [ ] `cargo test --workspace` succeeds.

## Tests

### Layer 2 — Event Handling
- [ ] `terminal_caps_detect_from_env` — capability detection still works
  after the move.

### Layer 3 — Rendering
- [ ] `tui_still_renders_after_merge` — a `TestBackend` draw completes with
  the merged crate structure.

### Layer 4 — Smoke
- [ ] `tmux_smoke_starts` — the binary launches in tmux without panic.

## Files touched

- `crates/runie-term/` (delete or archive)
- `crates/runie-tui/Cargo.toml`
- `crates/runie-tui/src/lib.rs`
- `crates/runie-tui/src/main.rs` (new binary entry)
- `Cargo.toml` workspace members

## Notes

If keeping a separate binary crate is desired, create a `runie-bin` crate
that depends only on `runie-tui`. But for now the simplest path is to move
all `runie-term` code into `runie-tui`.
