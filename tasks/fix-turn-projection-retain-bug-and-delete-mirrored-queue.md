# Fix turn projection retain bug and delete mirrored queue

## Status

`done`

## Context

`model/state/turn_projections.rs:144-148` and `:169-173` use inverted `retain` closures that drop all steering/follow-up messages except the one being delivered. Also, `AgentState` mirrors the turn message queue, creating drift risk.

## Goal

Fix the retain condition and delete the mirrored `AgentState.message_queue`, deriving `queue_count` from `TurnActor` facts.

## Acceptance Criteria

- [x] Fix retain to keep all messages except the delivered one.
- [x] Remove `AgentState.message_queue` duplicate.
- [x] Derive queue state from `TurnState` projection.
- [x] Multi-turn queue tests pass.

## Design Impact

No change to TUI element design or composition. Only queue behavior changes.

## Tests

- **Layer 1 — State/Logic:** Unit tests for steering/follow-up delivery and queue state.
- **Layer 2 — Event Handling:** `TurnActor` facts drive queue display.
- **Layer 3 — Rendering:** `TestBackend` queue indicator unchanged.
- **Layer 4 — E2E:** Provider replay fixture with queued messages passes.
- **Live tmux validation:** Queue multiple messages; deliver them and verify none are lost.

## Completion Validation

- [x] **Unit tests** — `cargo test --lib` covers the changed logic and all new/modified unit tests pass.
- [x] **E2E tests** — `cargo test --workspace` passes, including any new integration or provider-replay tests.
- [x] **Live tmux run tests** — the change is exercised in a real terminal tmux session (or a live CLI/headless scenario if the task does not affect the TUI).
