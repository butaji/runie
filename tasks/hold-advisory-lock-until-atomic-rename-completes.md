# Hold advisory lock until atomic rename completes

## Status

`done`

## Context

`crates/runie-core/src/io/atomic_write.rs:28-49` acquires an `fs2` advisory lock but drops it and deletes the lock file before the final `std::fs::rename`, leaving a race window for concurrent writers.

## Goal

Keep the lock (and lock file) alive until after `rename` succeeds.

## Acceptance Criteria
- [ ] Restructure function so lock is held through temp-file write and rename.
- [ ] Add concurrent stress test.
- [ ] Verify permissions remain `0o600`.

## Design Impact

No change to TUI element design or composition unless explicitly noted. Only implementation behavior, dependency graph, internal architecture, async runtime, or documentation changes.

## Tests

- **Layer 1 — State/Logic:** Concurrent stress test asserting final content is exactly one write.
- **Layer 2 — Event Handling:** N/A.
- **Layer 3 — Rendering:** N/A.
- **Layer 4 — E2E:** Trust/auth persistence tests pass.
- **Live tmux testing session (required):** N/A.

> **Live tmux testing session required:** After the implementation passes unit and E2E tests, run a real terminal tmux session that exercises the changed behavior. The task is not done until the live session succeeds.
## Completion Validation

- [ ] **Unit tests** — `cargo test --lib` covers the changed logic and all new/modified unit tests pass.
- [ ] **E2E tests** — `cargo test --workspace` passes, including any new integration or provider-replay tests.
- [ ] **Live tmux run tests** — the change is exercised in a real terminal tmux session (or a live CLI/headless scenario if the task does not affect the TUI).
