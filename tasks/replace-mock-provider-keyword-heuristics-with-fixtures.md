# Replace mock provider keyword heuristics with fixtures

## Status

`todo`

## Context

`crates/runie-provider/src/mock.rs:71-217` decides responses by substring-matching prompts (`"list files"`, `"read"`, `"native tool"`, etc.) and emits legacy `TOOL:` / XML / JSON formats. This is brittle and tests the keyword matcher more than the agent.

## Goal

Delete the heuristic engine. Drive `MockProvider` from a fixture manifest mapping input patterns → pre-recorded `ProviderEvent` sequences, or replace it with `ReplayProvider` + fixtures.

## Acceptance Criteria

- [ ] Remove keyword matching from `MockProvider`.
- [ ] Add fixture manifest and loader.
- [ ] Port existing tests to explicit fixtures.
- [ ] All agent tests pass.

## Design Impact

No change to TUI element design or composition. Only test provider behavior changes.

## Tests

- **Layer 1 — State/Logic:** Unit tests for fixture loading and selection.
- **Layer 2 — Event Handling:** Mock provider emits configured events.
- **Layer 3 — Rendering:** N/A.
- **Layer 4 — E2E:** Agent tests with mock provider pass.
- **Live tmux testing session (required):** N/A.

> **Live tmux testing session required:** After the implementation passes unit and E2E tests, run a real terminal tmux session that exercises the changed behavior. The task is not done until the live session succeeds.
## Completion Validation

- [ ] **Unit tests** — `cargo test --lib` covers the changed logic and all new/modified unit tests pass.
- [ ] **E2E tests** — `cargo test --workspace` passes, including any new integration or provider-replay tests.
- [ ] **Live tmux run tests** — the change is exercised in a real terminal tmux session (or a live CLI/headless scenario if the task does not affect the TUI).

### SSOT/Event Compliance
- [ ] **Actor/SSOT:** `ProviderActor` owns provider state; fixture-driven mock replaces heuristic.
- [ ] **Trigger events:** N/A (test fixture change doesn't introduce state transitions).
- [ ] **Observer events:** N/A (test fixture change doesn't emit events).
- [ ] **No direct mutations:** N/A (test fixture change doesn't mutate state).
- [ ] **No new mirrors:** N/A (test fixtures are test data, not authoritative state).
- [ ] **Async work observed:** N/A (test fixtures are synchronous).
