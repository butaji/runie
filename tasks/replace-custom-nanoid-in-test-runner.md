# Replace custom nanoid in test runner

**Status**: done
**Note**: Verified 2026-06-29 — `nanoid()` in runner.rs now uses `uuid::Uuid::new_v4()`.
**Milestone**: R6
**Category**: Test harness
**Priority**: P3

**Depends on**: centralize-test-fixtures-and-mocks
**Blocks**: none

## Description

`crates/runie-testing/src/runner.rs` builds IDs from a hash of the current time instead of using a real nanoid/uuid crate. Use `uuid::new_v4()` (already a workspace dep) or add `nanoid`.

## Acceptance Criteria

- [x] Replace the custom ID generation with `uuid::new_v4()` or `nanoid`.
- [x] Ensure IDs remain unique and stable enough for test artifacts.
- [x] `cargo test --workspace` succeeds after the change.
- [x] `cargo check --workspace` succeeds with no new warnings.

## Tests

### Layer 1 — State/Logic
- [x] `runner_id_is_unique` — generated IDs are unique across calls.

### Layer 2 — Event Handling
- [x] N/A.

### Layer 3 — Rendering
- [x] N/A.

### Layer 4 — Provider Replay / Mock-Tool E2E
- [x] N/A.

## Files touched

- `crates/runie-testing/src/runner.rs`
- `crates/runie-testing/Cargo.toml`

## Notes

- Low priority; IDs only need uniqueness within a test run.
