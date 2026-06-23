# Define Approval Defaults for Headless/Server Modes

**Status**: done
**Milestone**: R4
**Category**: Architecture / Security
**Priority**: P3

**Depends on**: `permission-system-runtime-wiring`
**Blocks**: none

## Description

Headless/server modes execute tools without any approval mechanism, consistent with the missing permission integration in `turn.rs`. Once permission integration is fixed, non-interactive modes need a safe default.

## Acceptance Criteria

- [x] Non-interactive modes deny destructive tools by default.
- [x] Provide an explicit `--yolo` or similar flag to auto-approve (with logging).
- [x] `cargo test --workspace` succeeds.

## Tests

### Layer 1 — State/Logic
- [x] `headless_default_denies_destructive_tool` — write_file denied without `--yolo`.
- [x] `headless_yolo_allows_destructive_tool` — flag restores auto-approval.

### Layer 2 — Event Handling
N/A.

### Layer 3 — Rendering
N/A.

### Layer 4 — Smoke / Crash
N/A.

## Files touched

- `crates/runie-json/src/main.rs`
- `crates/runie-server/src/main.rs`
- `crates/runie-core/src/config.rs`

## Notes

Blocked on `permission-system-runtime-wiring`.
