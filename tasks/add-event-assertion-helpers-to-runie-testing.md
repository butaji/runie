# Add event assertion helpers to runie-testing

## Status

`todo`

## Context

Tests repeatedly create `Arc<Mutex<Vec<Event>>>` closures and manually filter/count events. This boilerplate is duplicated across agent tests.

## Goal

Add helpers to `runie-testing`:
- `capture_events() -> (Arc<Mutex<Vec<Event>>>, EmitFn)`
- `assert_event(events, predicate)`
- `count_events(events, predicate) -> usize`

## Acceptance Criteria

- [ ] Add helpers to `runie-testing`.
- [ ] Replace manual event assertions in tests.
- [ ] All tests pass.

## Design Impact

No change to TUI element design or composition. Only test code changes.

## Tests

- **Layer 1 — State/Logic:** N/A.
- **Layer 2 — Event Handling:** N/A.
- **Layer 3 — Rendering:** N/A.
- **Layer 4 — E2E:** All tests pass.
- **Live tmux validation:** N/A.

## Completion Validation

- [ ] **Unit tests** — `cargo test --lib` covers the changed logic and all new/modified unit tests pass.
- [ ] **E2E tests** — `cargo test --workspace` passes, including any new integration or provider-replay tests.
- [ ] **Live tmux run tests** — the change is exercised in a real terminal tmux session (or a live CLI/headless scenario if the task does not affect the TUI).
