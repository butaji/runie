# Adopt testing helpers across all tests

## Status

`todo`

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
- **Live tmux validation:** N/A.

## Completion Validation

- [ ] **Unit tests** — `cargo test --lib` covers the changed logic and all new/modified unit tests pass.
- [ ] **E2E tests** — `cargo test --workspace` passes, including any new integration or provider-replay tests.
- [ ] **Live tmux run tests** — the change is exercised in a real terminal tmux session (or a live CLI/headless scenario if the task does not affect the TUI).
