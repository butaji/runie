# Consolidate login flow handlers into `login_flow/`

**Status**: todo
**Milestone**: R4
**Category**: Core / State
**Priority**: P1

**Depends on**: none
**Blocks**: none

## Description

The login flow is split across two roots: `login_flow/` (state machine, panels, validation — the "what") and `update/login_flow.rs` (event handlers — the "how"). The handlers file (405 LOC) imports heavily from `login_flow/` and its tests live in `update/login_flow/tests.rs`. Move `update/login_flow.rs` → `login_flow/handlers.rs` and its tests alongside, so the entire login-flow concept lives in one directory. This is orthogonal to `consolidate-dual-path-modules` (which explicitly excludes login_flow) and `flatten-update-login-flow-tests-dir` (which this task absorbs).

## Acceptance Criteria

- [ ] `update/login_flow.rs` and `update/login_flow/` removed.
- [ ] New `login_flow/handlers.rs` contains the event handler functions.
- [ ] `login_flow/mod.rs` declares `mod handlers;` and re-exports `login_flow_event`, `login_flow_cancel`, `login_flow_start`.
- [ ] `update/mod.rs` no longer declares `mod login_flow;`.
- [ ] `update/mod.rs` dispatcher calls `crate::login_flow::handlers::login_flow_event` (or re-export).
- [ ] `state.clone()` / `flow.clone()` sites in the moved file reduced: extract needed fields (`provider`, `key`) before the mutable borrow instead of cloning the whole `LoginFlowState`. Target ≤2 whole-struct clones (down from 5).
- [ ] All `tests/login_logout/` tests pass.
- [ ] `cargo test --workspace` succeeds.
- [ ] `cargo check --workspace` succeeds with no new warnings.

## Tests

### Layer 1 — State/Logic
- [ ] `login_flow_state_transitions` — existing state machine tests in `login_flow/state_tests.rs` still pass after move.

### Layer 2 — Event Handling
- [ ] `login_flow_start_opens_dialog` — `Event::LoginFlow(Start)` produces an open dialog.
- [ ] `login_flow_cancel_pops_panel` — Cancel at root closes the dialog.
- [ ] `login_flow_save_persists_provider` — Save with validated key + selected models emits ConfigMsg::SaveProvider.
- [ ] `login_flow_rejects_empty_key` — SubmitKey with empty key sets warning transient, no crash.

### Layer 3 — Rendering
- [ ] N/A — panel builders unchanged; rendering covered by existing TUI tests.

### Layer 4 — Smoke / Crash
- [ ] `tests/login_logout/*` suite (14 files) passes end-to-end.

## Files touched

- `crates/runie-core/src/update/login_flow.rs` → delete (move to `login_flow/handlers.rs`)
- `crates/runie-core/src/update/login_flow/tests.rs` → delete (move to `login_flow/handlers_tests.rs` or inline)
- `crates/runie-core/src/update/login_flow/` → delete dir
- `crates/runie-core/src/login_flow/handlers.rs` → new
- `crates/runie-core/src/login_flow/mod.rs` — add `mod handlers;` + re-exports
- `crates/runie-core/src/update/mod.rs` — remove `mod login_flow;`, update dispatcher call

## Notes

Absorbs `flatten-update-login-flow-tests-dir`. Do this task INSTEAD of that one. The handlers file (405 LOC) is under the 500-line limit but close — after the move, if it grows, consider splitting by step (provider_picker, key_input, model_select handlers). `consolidate-dual-path-modules` handles `login_config` (a different module) and does NOT conflict.
