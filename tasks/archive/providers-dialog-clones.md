# Reduce Unnecessary Cloning in Providers Dialog

**Status**: done
**Milestone**: R3
**Category**: TUI / Rendering
**Priority**: P3

**Depends on**: none
**Blocks**: none

## Description

`providers_dialog.rs` clones model strings into dialog items even when they could be borrowed or referenced.

## Acceptance Criteria

- [x] Accept `String` only where required by the `Panel` API; otherwise pass `&str`.
- [x] `cargo test --workspace` succeeds.

## Tests

N/A — minor performance cleanup.

## Files touched

- `crates/runie-core/src/providers_dialog.rs`

## Notes

Low-impact polish.
