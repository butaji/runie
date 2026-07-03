# Sweep redundant `.to_string()` allocations

**Status**: done
**Milestone**: R4
**Category**: Core / State
**Priority**: P3

**Depends on**: none
**Blocks**: none

## Description

`.to_string()` appears in many places on `&str` values — a mechanical allocation that can be replaced with `.to_owned()`. Also, `clippy::redundant_clone` flags unnecessary clones.

## Acceptance Criteria

- [x] All `String.to_string()` redundant clones removed (grep finds zero `: String = .*\.to_string\(\)` where the receiver is already `String`).
- [x] `&str.to_string()` in `String`-typed positions converted to `.into()` or `.to_owned()`.
- [x] No `to_string()` on `String` receivers remains in production code.
- [x] `cargo clippy --workspace` passes (`clippy::str_to_string`, `clippy::redundant_clone`).
- [x] `cargo test --workspace` succeeds.

## Tests

### Layer 1 — State/Logic
- [x] N/A — no logic change, only allocation removal.

### Layer 2 — Event Handling
- [x] N/A — no event handling change.

### Layer 3 — Rendering
- [x] N/A — no rendering change.

### Layer 4 — Smoke / Crash
- [x] `cargo test --workspace` green confirms no behavior change from the sweep.

## Files touched

- Workspace-wide sweep across all crates.
- `Cargo.toml` - no changes (lints not permanently enabled as workspace has no root package).

## Notes

- Used `cargo clippy --fix -- -W clippy::str_to_string -W clippy::redundant_clone` across the workspace to mechanically apply fixes.
- `str_to_string`: replaced `&str.to_string()` with `.to_owned()` (453 instances → 0 warnings).
- `redundant_clone`: removed unnecessary `.clone()` calls on `Copy` types (4 instances → 0 warnings).
- Remaining clippy warnings in the workspace are unrelated to this task.
- The lint flags were applied via command-line during the fix pass; they are not permanently enabled because the workspace is a virtual manifest without a root package.
