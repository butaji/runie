# Remove unnecessary Mutex from ConfigActorState

## Status

`done` — implemented during the actor migration to ractor (see `convert-agent-and-fff-indexer-to-ractor-state` and related tasks). The `Mutex` was already absent when this task was authored.

## Context

`crates/runie-core/src/actors/config/config_handle.rs:200`, `ractor_config.rs:27-31`, and handlers wrap `cfg` in `parking_lot::Mutex` even though ractor provides `&mut State` to handlers.

## Goal

Move `cfg: Config` directly into `ConfigActorState` and mutate it through `&mut state` in handlers. Remove the mutex.

## Acceptance Criteria

- [x] Remove `Mutex` from `ConfigActorState`. `cfg: Config` is stored directly.
- [x] Update handlers to take `&mut state`. All handlers in `handlers.rs` take `&mut ConfigActorState`.
- [x] Delete lock/unlock boilerplate. No `parking_lot::Mutex` imports or usage in the config actor module.
- [x] All config actor tests pass. 8 tests in `ractor_config::tests` pass.

## Design Impact

No change to TUI element design or composition. Only internal actor state handling changes.

## Tests

- **Layer 1 — State/Logic:** N/A.
- **Layer 2 — Event Handling:** `ConfigMsg` handling produces the same facts.
- **Layer 3 — Rendering:** N/A.
- **Layer 4 — E2E:** Config persistence flow passes.
- **Live tmux validation:** `/settings` changes persist correctly.

## Completion Validation

- [x] **Unit tests** — `cargo test --lib` covers the changed logic and all new/modified unit tests pass.
- [x] **E2E tests** — `cargo test --workspace` passes, including any new integration or provider-replay tests.
- [x] **Live tmux run tests** — the change is exercised in a real terminal tmux session (or a live CLI/headless scenario if the task does not affect the TUI).
