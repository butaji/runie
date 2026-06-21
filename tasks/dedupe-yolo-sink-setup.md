# Dedupe yolo/approval-sink setup

**Status**: todo
**Milestone**: R4
**Category**: Architecture / Actors
**Priority**: P2

**Depends on**: none
**Blocks**: none

## Description

`crates/runie-json/src/main.rs` and `crates/runie-server/src/main.rs` both parse `--yolo`, print the same warning, and construct `Arc::new(AutoAllowSink)` or `Arc::new(DenyAllSink)`. Wording and default behavior can drift between CLIs.

## Acceptance Criteria

- [ ] A helper `build_sink(yolo: bool) -> Arc<dyn ApprovalSink>` (or similar) lives in `runie-agent` or `runie-core`.
- [ ] `runie-json` and `runie-server` use the helper.
- [ ] `runie-print` is also covered if it has similar logic.
- [ ] `cargo test --workspace` succeeds.

## Tests

### Layer 1 — State/Logic
- [ ] `build_sink_yolo_true_allows_all` — returns auto-allow sink.
- [ ] `build_sink_yolo_false_denies_all` — returns deny-all sink.

### Layer 2 — Event Handling
- [ ] N/A.

### Layer 3 — Rendering
- [ ] N/A.

### Layer 4 — Provider Replay / Mock-Tool E2E
- [ ] `headless_yolo_turn_allows_tools` — a headless turn with `--yolo` executes tools without prompting.

## Files touched

- `crates/runie-json/src/main.rs`
- `crates/runie-server/src/main.rs`
- `crates/runie-print/src/main.rs`
- `crates/runie-agent/src/lib.rs` or `crates/runie-core/src/permissions/mod.rs`

## Notes

Combine with `collapse-headless-binaries-into-one-cli` if that task lands first; the helper becomes a CLI flag handler.
