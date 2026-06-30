# Move ApprovalRegistry into PermissionActor state

## Status

`todo`

## Context

`crates/runie-core/src/permissions/approval_registry.rs` holds pending permission replies in a `parking_lot::Mutex<HashMap<...>>`. `RactorPermissionActor` serializes all messages, so the mutex is unnecessary and a state-ownership leak.

## Goal

Move the pending-reply map into `PermissionActorState` and delete the `ApprovalRegistry` wrapper.

## Acceptance Criteria

- [ ] Add pending map field to `PermissionActorState`.
- [ ] Remove `ApprovalRegistry` module.
- [ ] Update `RactorPermissionActor` handlers to use state.
- [ ] All permission/approval tests pass.

## Design Impact

No change to TUI element design or composition. Only internal permission actor state changes.

## Tests

- **Layer 1 — State/Logic:** Unit tests for register/resolve/cancel in actor state.
- **Layer 2 — Event Handling:** `PermissionRequest`/`PermissionResponse` events unchanged.
- **Layer 3 — Rendering:** Permission dialog snapshots match.
- **Layer 4 — E2E:** Provider replay fixture with approvals passes.
- **Live tmux validation:** Approve/deny a tool call; response arrives correctly.

## Completion Validation

- [ ] **Unit tests** — `cargo test --lib` covers the changed logic and all new/modified unit tests pass.
- [ ] **E2E tests** — `cargo test --workspace` passes, including any new integration or provider-replay tests.
- [ ] **Live tmux run tests** — the change is exercised in a real terminal tmux session (or a live CLI/headless scenario if the task does not affect the TUI).
