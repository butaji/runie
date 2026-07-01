# Rewrite turn projections tests to not mutate AgentState

## Status

`done`

## Context

`crates/runie-core/src/model/state/turn_projections.rs:7-196` contains tests that directly set `state.agent_state_mut().turn_active = true` and overwrite `*state.agent_state_mut() = AgentState::from(&state.turn_state)`, violating the projection invariant.

## Goal

Rewrite tests to seed `TurnState` and assert the projection via `AgentState::from(&turn_state)`.

## Acceptance Criteria
- [ ] Remove direct `AgentState` mutation in tests.
- [ ] Assert projection behavior.
- [ ] All tests pass.

## Design Impact

No change to TUI element design or composition unless explicitly noted. Only implementation behavior, dependency graph, internal architecture, async runtime, or documentation changes.

## Tests

- **Layer 1 — State/Logic:** Unit tests for projection from TurnState.
- **Layer 2 — Event Handling:** N/A.
- **Layer 3 — Rendering:** N/A.
- **Layer 4 — E2E:** `cargo test -p runie-core turn_projections` passes.
- **Live tmux testing session (required):** N/A.

> **Live tmux testing session required:** After the implementation passes unit and E2E tests, run a real terminal tmux session that exercises the changed behavior. The task is not done until the live session succeeds.
## Completion Validation

- [ ] **Unit tests** — `cargo test --lib` covers the changed logic and all new/modified unit tests pass.
- [ ] **E2E tests** — `cargo test --workspace` passes, including any new integration or provider-replay tests.
- [ ] **Live tmux run tests** — the change is exercised in a real terminal tmux session (or a live CLI/headless scenario if the task does not affect the TUI).
