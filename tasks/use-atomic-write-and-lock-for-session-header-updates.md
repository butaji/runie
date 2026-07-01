# Use atomic write and lock for session header updates

## Status

`todo`

## Context

`crates/runie-core/src/session/store.rs:203-220` and `header.rs:39-47` rewrite the session JSONL header without holding the store's advisory lock and without atomic rename.

## Goal

Acquire `exclusive_lock` and use `crate::io::atomic_write` for header updates.

## Acceptance Criteria
- [ ] Lock the store file before header rewrite.
- [ ] Use temp-file + rename via `atomic_write`.
- [ ] Stress-test concurrent `append` and `update_metadata`.

## Design Impact

No change to TUI element design or composition unless explicitly noted. Only implementation behavior, dependency graph, internal architecture, async runtime, or documentation changes.

## Tests

- **Layer 1 — State/Logic:** Stress test for concurrent append/metadata update.
- **Layer 2 — Event Handling:** N/A.
- **Layer 3 — Rendering:** N/A.
- **Layer 4 — E2E:** Session persistence tests pass.
- **Live tmux validation:** Save/load sessions repeatedly.

## Completion Validation

- [ ] **Unit tests** — `cargo test --lib` covers the changed logic and all new/modified unit tests pass.
- [ ] **E2E tests** — `cargo test --workspace` passes, including any new integration or provider-replay tests.
- [ ] **Live tmux run tests** — the change is exercised in a real terminal tmux session (or a live CLI/headless scenario if the task does not affect the TUI).
