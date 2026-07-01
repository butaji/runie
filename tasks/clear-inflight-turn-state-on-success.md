# Clear in-flight turn state on success

## Status

`todo`

## Context

`crates/runie-agent/src/actor.rs:123-187` stores `current_turn_token`, `current_turn_gate`, and `current_turn_handle` for abort/cancel, but the spawned task never clears them after normal completion. Every subsequent `AgentMsg::Run` is rejected.

## Goal

Clear the in-flight token/handle/gate after the turn task finishes successfully.

## Acceptance Criteria
- [ ] Detect turn completion in the spawned task.
- [ ] Send a completion fact or directly clear actor state.
- [ ] Add a Layer-2/Layer-4 test for two sequential turns.

## Design Impact

No change to TUI element design or composition unless explicitly noted. Only implementation behavior, dependency graph, internal architecture, async runtime, or documentation changes.

## Tests

- **Layer 1 — State/Logic:** Unit test for sequential turns.
- **Layer 2 — Event Handling:** Actor accepts a second Run after first completes.
- **Layer 3 — Rendering:** N/A.
- **Layer 4 — E2E:** Provider replay with two sequential turns passes.
- **Live tmux validation:** Multi-turn chat works.

## Completion Validation

- [ ] **Unit tests** — `cargo test --lib` covers the changed logic and all new/modified unit tests pass.
- [ ] **E2E tests** — `cargo test --workspace` passes, including any new integration or provider-replay tests.
- [ ] **Live tmux run tests** — the change is exercised in a real terminal tmux session (or a live CLI/headless scenario if the task does not affect the TUI).
