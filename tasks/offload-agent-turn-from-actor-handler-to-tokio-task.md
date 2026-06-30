# Offload agent turn from actor handler to a tokio task

## Status

`todo`

## Context

`crates/runie-agent/src/actor.rs:96-151` runs the entire provider turn (streaming + tool loop) inside `RactorAgentActor::handle`. Because ractor processes messages serially, the actor mailbox is blocked for the whole turn. Abort is currently detected via a separate event-bus subscription, which is fragile.

## Goal

Spawn `run_agent_turn(...)` as a `tokio::spawn` task; store its `AbortHandle` (or a per-turn `CancellationToken`) in `AgentActorState`; add an `AgentMsg::Abort` handler that aborts the in-flight turn.

## Acceptance Criteria

- [ ] `handle` returns immediately after spawning the turn task.
- [ ] `AgentActorState` holds the current turn `AbortHandle`/`CancellationToken`.
- [ ] `AgentMsg::Abort` cancels the in-flight turn.
- [ ] Remove the side event-bus subscription used for abort detection.
- [ ] Existing turn lifecycle events still flow through the event bus.

## Design Impact

No change to TUI element design or composition. Only internal agent concurrency behavior changes.

## Tests

- **Layer 1 — State/Logic:** Unit tests for abort token propagation and cleanup.
- **Layer 2 — Event Handling:** `AgentMsg::Abort` cancels a streaming turn and emits `TurnAborted`.
- **Layer 3 — Rendering:** `TestBackend` shows the turn aborted state.
- **Layer 4 — E2E:** Provider replay fixture cancels a turn mid-stream.
- **Live tmux validation:** Start a turn, press the abort shortcut, and confirm the UI returns to idle immediately.

## Completion Validation

- [ ] **Unit tests** — `cargo test --lib` covers the changed logic and all new/modified unit tests pass.
- [ ] **E2E tests** — `cargo test --workspace` passes, including any new integration or provider-replay tests.
- [ ] **Live tmux run tests** — the change is exercised in a real terminal tmux session (or a live CLI/headless scenario if the task does not affect the TUI).
