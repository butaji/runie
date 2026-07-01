# Graceful leader shutdown with JoinSet timeout

## Status

`todo`

## Context

`crates/runie-core/src/actors/leader/actor.rs:122-133` and `handle.rs:111-133` spawn coordinator/TCP tasks but do not store handles; shutdown kills actors and awaits sequentially.

## Goal

Store handles, send graceful stops, and await them in parallel with `tokio::time::timeout`.

## Acceptance Criteria
- [ ] Return/store coordinator and TCP handles.
- [ ] Use `JoinSet` or `tokio::join!` with timeout.
- [ ] Send graceful stop before await.

## Design Impact

No change to TUI element design or composition unless explicitly noted. Only implementation behavior, dependency graph, internal architecture, or async runtime changes.

## Tests

- **Layer 1 — State/Logic:** N/A.
- **Layer 2 — Event Handling:** N/A.
- **Layer 3 — Rendering:** N/A.
- **Layer 4 — E2E:** Leader shutdown tests pass.
- **Live tmux validation:** Server mode exits cleanly.

## Completion Validation

- [ ] **Unit tests** — `cargo test --lib` covers the changed logic and all new/modified unit tests pass.
- [ ] **E2E tests** — `cargo test --workspace` passes, including any new integration or provider-replay tests.
- [ ] **Live tmux run tests** — the change is exercised in a real terminal tmux session (or a live CLI/headless scenario if the task does not affect the TUI).
