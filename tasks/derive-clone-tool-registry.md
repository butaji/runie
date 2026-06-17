# Add `#[derive(Clone)]` to ToolRegistry

**Status**: done
**Completed**: 2026-06-16
**Notes**: Replaced manual `Clone` impl with `#[derive(Clone)]` on `ToolRegistry` and removed the manual impl from `actors/subagent.rs`. cargo test --workspace passes.
**Milestone**: R4
**Category**: Tools
**Priority**: P2

**Depends on**: (none)
**Blocks**: (none)

## Description

Replace manual `Clone` implementation with `#[derive(Clone)]` on `ToolRegistry`.

**Location:** `crates/runie-core/src/tool/mod.rs` or `crates/runie-core/src/actors/subagent.rs:313-319`

`HashMap` and `Arc` are both cloneable, so `#[derive(Clone)]` should work.

## Acceptance Criteria

- [ ] Manual `Clone` impl removed.
- [ ] `#[derive(Clone)]` added to `ToolRegistry` struct.
- [ ] `cargo test --workspace` succeeds.

## Tests

### Layer 1 — State/Logic
N/A (cosmetic change).

### Layer 2 — Event Handling
N/A.

### Layer 3 — Rendering
N/A.

### Layer 4 — Smoke / Crash
N/A.

## Files touched

- `crates/runie-core/src/tool/mod.rs`

## Notes

Quick fix, 2 minutes.
