# Delete runie-term-archive Crate

**Status**: done
**Milestone**: R3
**Category**: TUI / Rendering
**Priority**: P0

**Depends on**: (none)
**Blocks**: (none)

## Description

`crates/runie-term-archive/` is a stale near-duplicate of `runie-tui` (5,296 lines, not a workspace member, declares package `runie-term` and bin `runie`). It shadows the active crate and forces every fix to be mirrored or silently diverges.

## Acceptance Criteria

- [ ] Any unique files (`tests/render.rs`, `tests/terminal_setup.rs`) are ported or explicitly discarded.
- [ ] `crates/runie-term-archive/` directory is deleted.
- [ ] No remaining path references in docs, scripts, or Cargo workspace.
- [ ] `cargo test --workspace` succeeds.

## Tests

### Layer 2 — Event Handling
- [ ] `runie_tui_tests_still_run` — all `runie-tui` tests pass after archive deletion.

### Layer 4 — Smoke
- [ ] `tmux_smoke_starts` — `scripts/smoke-tmux.sh` still launches the binary.

## Files touched

- `crates/runie-term-archive/` (deleted)
- `crates/runie-tui/src/tests/render.rs` (if ported)
- `docs/SPEC.md`
- `docs/CRATE_DECISIONS.md`

## Notes

`merge-runie-term-into-tui.md` was previously marked done but the archive crate was left behind.
