# Replace vacuous print mode test with replay test

## Status

`todo`

## Context

`crates/runie-cli/src/print.rs:29-35` asserts `result.is_err() || result.is_ok()`, which is always true. It requires a real provider/config.

## Goal

Replace with a deterministic replay test of `HeadlessEvent` formatting or prompt propagation.

## Acceptance Criteria
- [ ] Delete vacuous test.
- [ ] Add replay-based test using `MockProvider`/`ReplayProvider`.
- [ ] No real provider needed.

## Design Impact

No change to TUI element design or composition unless explicitly noted. Only implementation behavior, dependency graph, internal architecture, async runtime, or documentation changes.

## Tests

- **Layer 1 — State/Logic:** N/A.
- **Layer 2 — Event Handling:** N/A.
- **Layer 3 — Rendering:** N/A.
- **Layer 4 — E2E:** New print test passes.
- **Live tmux validation:** N/A.

## Completion Validation

- [ ] **Unit tests** — `cargo test --lib` covers the changed logic and all new/modified unit tests pass.
- [ ] **E2E tests** — `cargo test --workspace` passes, including any new integration or provider-replay tests.
- [ ] **Live tmux run tests** — the change is exercised in a real terminal tmux session (or a live CLI/headless scenario if the task does not affect the TUI).
