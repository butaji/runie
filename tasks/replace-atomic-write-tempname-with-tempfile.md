# Replace atomic_write temp-name generator with tempfile

## Status

`todo`

## Context

`crates/runie-core/src/persistence.rs` uses a hand-rolled temp-name generator mixing stack address, nanoseconds, and PID.

## Goal

Use `tempfile::NamedTempFile::new_in(parent)` for atomic writes.

## Acceptance Criteria
- [ ] Replace custom temp-name logic.
- [ ] Preserve `fs2` lock and `0o600` permissions.
- [ ] Cross-platform behavior unchanged.

## Design Impact

No change to TUI element design or composition unless explicitly noted. Only implementation behavior, dependency graph, internal architecture, or documentation changes.

## Tests

- **Layer 1 — State/Logic:** Unit tests for atomic write and permissions.
- **Layer 2 — Event Handling:** N/A.
- **Layer 3 — Rendering:** N/A.
- **Layer 4 — E2E:** Persistence tests pass.
- **Live tmux validation:** N/A.

## Completion Validation

- [ ] **Unit tests** — `cargo test --lib` covers the changed logic and all new/modified unit tests pass.
- [ ] **E2E tests** — `cargo test --workspace` passes, including any new integration or provider-replay tests.
- [ ] **Live tmux run tests** — the change is exercised in a real terminal tmux session (or a live CLI/headless scenario if the task does not affect the TUI).
