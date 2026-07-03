# Dedupe yolo/approval-sink setup

**Status**: done
**Milestone**: R4
**Category**: Architecture / Actors
**Priority**: P2

**Depends on**: none
**Blocks**: none

## Description

`crates/runie-json/src/main.rs` and `crates/runie-server/src/main.rs` both parse `--yolo`, print the same warning, and construct `Arc::new(AutoAllowSink)` or `Arc::new(DenyAllSink)`. Wording and default behavior can drift between CLIs.

## Acceptance Criteria

- [x] A helper `build_sink(yolo: bool) -> Arc<dyn ApprovalSink>` lives in `runie-core/src/permissions/mod.rs`.
- [x] `runie-json` and `runie-server` use the helper.
- [x] `runie-print` is also covered (uses `build_sink(false)` since it has no yolo flag).
- [x] `cargo test --workspace` succeeds.

## Tests

### Layer 1 — State/Logic
- [x] `build_sink_yolo_true_allows_all` — returns auto-allow sink.
- [x] `build_sink_yolo_false_denies_all` — returns deny-all sink.

### Layer 2 — Event Handling
- N/A.

### Layer 3 — Rendering
- N/A.

### Layer 4 — Provider Replay / Mock-Tool E2E
- N/A.

## Files touched

- `crates/runie-core/src/permissions/mod.rs` — added `build_sink` helper
- `crates/runie-core/src/permissions/tests.rs` — added Layer 1 tests
- `crates/runie-json/src/main.rs` — use `build_sink` helper
- `crates/runie-server/src/main.rs` — use `build_sink` helper (via `headless_sink`)
- `crates/runie-print/src/main.rs` — use `build_sink(false)`

## Notes

The `--yolo` flag parsing and warning message remain in each binary's main function - those are CLI concerns. The sink construction is now shared.
