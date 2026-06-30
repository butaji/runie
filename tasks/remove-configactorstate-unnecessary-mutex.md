# Remove unnecessary Mutex from ConfigActorState

## Status

`todo`

## Context

`crates/runie-core/src/actors/config/config_handle.rs:200`, `ractor_config.rs:27-31`, and handlers wrap `cfg` in `parking_lot::Mutex` even though ractor provides `&mut State` to handlers.

## Goal

Move `cfg: Config` directly into `ConfigActorState` and mutate it through `&mut state` in handlers. Remove the mutex.

## Acceptance Criteria

- [ ] Remove `Mutex` from `ConfigActorState`.
- [ ] Update handlers to take `&mut state`.
- [ ] Delete lock/unlock boilerplate.
- [ ] All config actor tests pass.

## Design Impact

No change to TUI element design or composition. Only internal actor state handling changes.

## Tests

- **Layer 1 — State/Logic:** N/A.
- **Layer 2 — Event Handling:** `ConfigMsg` handling produces the same facts.
- **Layer 3 — Rendering:** N/A.
- **Layer 4 — E2E:** Config persistence flow passes.
- **Live tmux validation:** `/settings` changes persist correctly.

## Completion Validation

- [ ] **Unit tests** — `cargo test --lib` covers the changed logic and all new/modified unit tests pass.
- [ ] **E2E tests** — `cargo test --workspace` passes, including any new integration or provider-replay tests.
- [ ] **Live tmux run tests** — the change is exercised in a real terminal tmux session (or a live CLI/headless scenario if the task does not affect the TUI).
