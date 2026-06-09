# Remove dead code from ship review #2

**Status**: done

**Milestone**: MVP

**Category**: Architecture / Cleanup

## Description

SHIP_REVIEW_2 identified ~24.8% of runie-core as unused dead code. Remove it to reduce compile times, eliminate confusion, and align the codebase with the actual runtime architecture.

## Acceptance Criteria

- [x] Delete `crates/runie-core/src/event_bus.rs`
- [x] Delete `crates/runie-core/src/orchestrator.rs`
- [x] Delete `crates/runie-core/src/actors/` directory
- [x] Delete `crates/runie-core/src/session_jsonl.rs`
- [x] Delete `crates/runie-core/src/session_manager/` directory
- [x] Remove all corresponding `pub mod` and `pub use` declarations from `lib.rs`
- [x] Remove `render_generation` field from `AppState` and its initializer
- [x] Ensure `cargo test` still passes (no regressions)
- [x] Ensure `cargo build` still passes

## Tests

- Layer 1 — Existing state/logic tests must continue to pass
- Layer 2 — Existing event handling tests must continue to pass
- Layer 3 — Existing rendering tests must continue to pass
- Layer 4 — N/A (no async/event logic changes, only deletion)
