# Route agent abort through actor message, not event bus

## Status

`todo`

## Context

`crates/runie-agent/src/actor.rs:127-149` detects abort by subscribing to the event bus and waiting for `Event::TurnAborted`. Once the agent turn is offloaded to a task, this side channel becomes unnecessary.

## Goal

After offloading the turn, handle `AgentMsg::Abort` directly in the actor and cancel the per-turn token/handle. Remove the event-bus subscriber.

## Acceptance Criteria

- [ ] Add `AgentMsg::Abort` variant.
- [ ] Cancel the in-flight turn task in the handler.
- [ ] Remove event-bus subscription for abort.
- [ ] All abort tests pass.

## Design Impact

No change to TUI element design or composition. Only abort routing behavior changes.

## Tests

- **Layer 1 — State/Logic:** Unit test for abort message handling.
- **Layer 2 — Event Handling:** `AgentMsg::Abort` emits `TurnAborted`.
- **Layer 3 — Rendering:** `TestBackend` shows aborted state.
- **Layer 4 — E2E:** Provider replay fixture aborts mid-turn.
- **Live tmux validation:** Abort a turn with the shortcut; UI returns to idle.

## Completion Validation

- [ ] **Unit tests** — `cargo test --lib` covers the changed logic and all new/modified unit tests pass.
- [ ] **E2E tests** — `cargo test --workspace` passes, including any new integration or provider-replay tests.
- [ ] **Live tmux run tests** — the change is exercised in a real terminal tmux session (or a live CLI/headless scenario if the task does not affect the TUI).
