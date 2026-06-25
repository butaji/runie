# Consolidate login flow handlers into `login_flow/`

**Status**: done
**Milestone**: R4
**Category**: Core / State
**Priority**: P1

**Depends on**: none
**Blocks**: none

## Description

The login flow is split across two roots: `login_flow/` (state machine, panels, validation — the "what") and `update/login_flow.rs` (event handlers — the "how"). The handlers file (405 LOC) imports heavily from `login_flow/` and its tests live in `update/login_flow/tests.rs`. Move `update/login_flow.rs` → `login_flow/handlers.rs` and its tests alongside, so the entire login-flow concept lives in one directory. This is orthogonal to `consolidate-dual-path-modules` (which explicitly excludes login_flow) and `flatten-update-login-flow-tests-dir` (which this task absorbs).

## Acceptance Criteria

- [x] `update/login_flow.rs` and `update/login_flow/` removed.
- [x] New `login_flow/handlers.rs` contains the event handler functions.
- [x] `login_flow/mod.rs` declares `mod handlers;` and re-exports `login_flow_event`, `login_flow_cancel`, `login_flow_start`.
- [x] `update/mod.rs` no longer declares `mod login_flow;`.
- [x] `update/mod.rs` dispatcher calls `crate::login_flow::handlers::login_flow_event` (or re-export).
- [x] `state.clone()` / `flow.clone()` sites in the moved file reduced: extract needed fields (`provider`, `key`) before the mutable borrow instead of cloning the whole `LoginFlowState`. Target ≤2 whole-struct clones (down from 5).
- [x] All `tests/login_logout/` tests pass.
- [x] `cargo test --workspace` succeeds.
- [x] `cargo check --workspace` succeeds with no new warnings.

## Tests

### Layer 1 — State/Logic
- [x] `login_flow_state_transitions` — existing state machine tests in `login_flow/state_tests.rs` still pass after move.

### Layer 2 — Event Handling
- [x] `login_flow_start_opens_dialog` — `Event::LoginFlow(Start)` produces an open dialog.
- [x] `login_flow_cancel_pops_panel` — Cancel at root closes the dialog.
- [x] `login_flow_save_persists_provider` — Save with validated key + selected models emits ConfigMsg::SaveProvider.
- [x] `login_flow_rejects_empty_key` — SubmitKey with empty key sets warning transient, no crash.

### Layer 3 — Rendering
- [x] N/A — panel builders unchanged; rendering covered by existing TUI tests.

### Layer 4 — Smoke / Crash
- [x] `tests/login_logout/*` suite (14 files) passes end-to-end.

## Files touched

- `crates/runie-core/src/update/login_flow.rs` → delete (moved to `login_flow/handlers.rs`)
- `crates/runie-core/src/update/login_flow/tests.rs` → delete (moved to `login_flow/handlers_tests.rs`)
- `crates/runie-core/src/update/login_flow/` → delete dir
- `crates/runie-core/src/login_flow/handlers.rs` → new
- `crates/runie-core/src/login_flow/panel_ops.rs` → new (split from handlers to fix lint)
- `crates/runie-core/src/login_flow/mod.rs` — add `mod handlers;` + re-exports
- `crates/runie-core/src/update/mod.rs` — remove `mod login_flow;`, update dispatcher call

## Notes

Absorbs `flatten-update-login-flow-tests-dir`. The handlers file (405 LOC) was split into `handlers.rs` and `panel_ops.rs` to satisfy the 500-line file limit. All state/flow cloning was eliminated using `std::mem::take` pattern.
