# Rewrite AppState structural tests to not mutate AgentState

## Status

`todo`

## Context

`crates/runie-core/src/tests/appstate_structural.rs:137-152,199-207` sets `state.agent.streaming = true` and `state.agent.next_id = 42`, locking in the anti-pattern that `AgentState` is mutable.

## Goal

Rewrite tests to assert `AgentState` is a projection from `TurnState`; remove direct mutation assertions.

## Acceptance Criteria
- [ ] Remove direct `AgentState` mutations in tests.
- [ ] Assert projection from `TurnState`.
- [ ] All tests pass.

## Design Impact

No change to TUI element design or composition unless explicitly noted. Only implementation behavior, dependency graph, internal architecture, or async runtime changes.

## Tests

- **Layer 1 — State/Logic:** Unit tests for projection.
- **Layer 2 — Event Handling:** N/A.
- **Layer 3 — Rendering:** N/A.
- **Layer 4 — E2E:** `cargo test -p runie-core appstate` passes.
- **Live tmux validation:** N/A.

## Completion Validation

- [ ] **Unit tests** — `cargo test --lib` covers the changed logic and all new/modified unit tests pass.
- [ ] **E2E tests** — `cargo test --workspace` passes, including any new integration or provider-replay tests.
- [ ] **Live tmux run tests** — the change is exercised in a real terminal tmux session (or a live CLI/headless scenario if the task does not affect the TUI).
