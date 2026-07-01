# Cap and lock input history

## Status

`todo`

## Context

`crates/runie-core/src/input_history.rs` rewrites the JSONL file with `std::fs` and never caps size or takes a cross-process lock.

## Goal

Add `fs2` advisory lock, cap entries, and optionally migrate to the SQLite session store.

## Acceptance Criteria
- [ ] Lock history file during writes.
- [ ] Cap to a configurable max (default 1000).
- [ ] Atomic write or migration to session store.

## Design Impact

No change to TUI element design or composition unless explicitly noted. Only implementation behavior, dependency graph, or internal architecture changes.

## Tests

- **Layer 1 — State/Logic:** Unit tests for cap and lock behavior.
- **Layer 2 — Event Handling:** History-loaded fact unchanged.
- **Layer 3 — Rendering:** N/A.
- **Layer 4 — E2E:** Concurrent history writes do not corrupt file.
- **Live tmux validation:** History persists across TUI restarts.

## Completion Validation

- [ ] **Unit tests** — `cargo test --lib` covers the changed logic and all new/modified unit tests pass.
- [ ] **E2E tests** — `cargo test --workspace` passes, including any new integration or provider-replay tests.
- [ ] **Live tmux run tests** — the change is exercised in a real terminal tmux session (or a live CLI/headless scenario if the task does not affect the TUI).
