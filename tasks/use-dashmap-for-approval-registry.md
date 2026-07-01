# Use dashmap for approval registry

## Status

`todo`

## Context

`crates/runie-core/src/permissions/approval_registry.rs:15-19` wraps a `HashMap` in `parking_lot::Mutex` for concurrent permission reply registration/resolution.

## Goal

Replace with `dashmap::DashMap<String, oneshot::Sender<PermissionAction>>`.

## Acceptance Criteria
- [ ] Add `dashmap` dependency.
- [ ] Replace Mutex+HashMap.
- [ ] Ensure entries not held across awaits; no leaked senders on cancel.

## Design Impact

No change to TUI element design or composition unless explicitly noted. Only implementation behavior, dependency graph, internal architecture, async runtime, or documentation changes.

## Tests

- **Layer 1 — State/Logic:** Unit tests for concurrent register/resolve.
- **Layer 2 — Event Handling:** Permission facts still resolve correctly.
- **Layer 3 — Rendering:** N/A.
- **Layer 4 — E2E:** Permission gate tests pass.
- **Live tmux testing session (required):** Permission dialog still works.

> **Live tmux testing session required:** After the implementation passes unit and E2E tests, run a real terminal tmux session that exercises the changed behavior. The task is not done until the live session succeeds.
## Completion Validation

- [ ] **Unit tests** — `cargo test --lib` covers the changed logic and all new/modified unit tests pass.
- [ ] **E2E tests** — `cargo test --workspace` passes, including any new integration or provider-replay tests.
- [ ] **Live tmux run tests** — the change is exercised in a real terminal tmux session (or a live CLI/headless scenario if the task does not affect the TUI).
