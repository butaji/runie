# Remove TUI Mouse Target Duplicate

**Status**: done
**Milestone**: R3
**Category**: TUI / Rendering
**Priority**: P1

**Depends on**: (none)
**Blocks**: (none)

## Description

`runie-tui/src/ui/mouse.rs` defines `MouseTarget`, `compute_mouse_target`, `is_in_feed`, and `is_in_input`, but the authoritative versions live in `runie-core/src/snapshot.rs`. The TUI copy is unused and triggers dead-code warnings.

## Acceptance Criteria

- [ ] `crates/runie-tui/src/ui/mouse.rs` is deleted.
- [ ] TUI consumes `runie_core::snapshot::MouseTarget`/`compute_mouse_target` where needed.
- [ ] `cargo clippy --workspace` no longer warns about these items.

## Tests

### Layer 1 — State/Logic
- [ ] `mouse_target_enum_unchanged` — variant set still matches expected targets.

### Layer 3 — Rendering
- [ ] `mouse_click_still_targets` — any TUI mouse tests still pass.

## Files touched

- `crates/runie-tui/src/ui/mouse.rs` (deleted)
- `crates/runie-tui/src/ui/mod.rs`
- `crates/runie-tui/src/main.rs` (if it imports mouse.rs)

## Notes

None.
