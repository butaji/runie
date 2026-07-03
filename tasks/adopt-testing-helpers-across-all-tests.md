# Adopt testing helpers across all tests

## Status

`done`

## Context

Agent unit tests duplicate event-capture closures and filter/count assertions despite existing `runie_testing` helpers.

## Goal

Replace manual captures/assertions with `runie_testing::capture_events`, `assert_event`, `count_events`, and `find_event`.

## Acceptance Criteria
- [x] Replace duplicated closures in `runie-agent/src/tests/`.
- [x] Use assertion helpers everywhere.
- [x] All tests pass.

## Design Impact

No change to TUI element design or composition unless explicitly noted. Only implementation behavior, dependency graph, or internal architecture changes.

## Tests

- **Layer 1 — State/Logic:** N/A.
- **Layer 2 — Event Handling:** N/A.
- **Layer 3 — Rendering:** N/A.
- **Layer 4 — E2E:** `cargo test --workspace` passes.
- **Live tmux testing session (required):** N/A.

> **Live tmux testing session required:** After the implementation passes unit and E2E tests, run a real terminal tmux session that exercises the changed behavior. The task is not done until the live session succeeds.
## Completion Validation

- [x] **Unit tests** — `cargo test --lib` covers the changed logic and all new/modified unit tests pass.
- [x] **E2E tests** — `cargo test --workspace` passes, including any new integration or provider-replay tests.
- [x] **Live tmux run tests** — N/A (internal test infrastructure only).

## Implementation Notes

All agent tests now use helpers from `runie-testing`:
- `capture_events()` used in: `turn_gate.rs`, `turn.rs`, `tool_marker_state.rs`, `minimax_like.rs`, `openai_turn.rs`, `minimax_turn.rs`
- `count_events()` used for filtering TurnComplete, Done, ToolStart, ToolEnd, Thinking, ResponseDelta events
- `find_event()` used for finding ToolEnd events
- `assert_event()` available in `lib.rs` re-exports
- Helpers defined in `crates/runie-testing/src/event_helpers.rs` and `crates/runie-testing/src/replay_provider.rs`
