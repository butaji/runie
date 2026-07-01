# Add event assertion helpers to runie-testing

## Status

`done`

## Context

Tests repeatedly create `Arc<Mutex<Vec<Event>>>` closures and manually filter/count events. This boilerplate is duplicated across agent tests.

## Goal

Add helpers to `runie-testing`:
- `capture_events() -> (Arc<Mutex<Vec<Event>>>, EmitFn)`
- `assert_event(events, predicate)`
- `count_events(events, predicate) -> usize`

## Acceptance Criteria

- [x] Add helpers to `runie-testing`.
- [x] Replace manual event assertions in tests.
- [x] All tests pass.

## Design Impact

No change to TUI element design or composition. Only test code changes.

## Tests

- **Layer 1 — State/Logic:** N/A.
- **Layer 2 — Event Handling:** N/A.
- **Layer 3 — Rendering:** N/A.
- **Layer 4 — E2E:** All tests pass.
- **Live tmux testing session (required):** N/A.

> **Live tmux testing session required:** After the implementation passes unit and E2E tests, run a real terminal tmux session that exercises the changed behavior. The task is not done until the live session succeeds.
## Completion Validation

- [x] **Unit tests** — `cargo test --lib` covers the changed logic and all new/modified unit tests pass.
- [x] **E2E tests** — `cargo test --workspace` passes, including any new integration or provider-replay tests.
- [x] **Live tmux run tests** — N/A (internal test infrastructure only).

## Implementation Notes

Helpers implemented in `crates/runie-testing/src/event_helpers.rs`:
- `count_events()` — filters events by predicate, returns count
- `find_event()` — returns first matching event
- `assert_event()` — panics if no event matches predicate
- `capture_events()` — returns `(Arc<Mutex<Vec<Event>>>, EmitFn)` from `replay_provider.rs`
- All re-exported from `runie-testing/src/lib.rs`
- Used throughout `runie-agent/src/tests/` and `runie-agent/tests/`
