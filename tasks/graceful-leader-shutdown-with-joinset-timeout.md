# Graceful leader shutdown with JoinSet timeout

## Status

`done`

## Context

`crates/runie-core/src/actors/leader/actor.rs:122-133` and `handle.rs:111-133` spawn coordinator/TCP tasks but do not store handles; shutdown kills actors and awaits sequentially.

## Goal

Store handles, send graceful stops, and await them in parallel with `tokio::time::timeout`.

## Acceptance Criteria
- [x] Return/store coordinator and TCP handles.
- [x] Use `JoinSet` or `tokio::join!` with timeout.
- [x] Send graceful stop before await.

## Design Impact

No change to TUI element design or composition unless explicitly noted. Only implementation behavior, dependency graph, internal architecture, or async runtime changes.

## Tests

- **Layer 1 — State/Logic:** N/A.
- **Layer 2 — Event Handling:** N/A.
- **Layer 3 — Rendering:** N/A.
- **Layer 4 — E2E:** Leader shutdown tests pass.
- **Live tmux testing session (required):** Server mode exits cleanly.

> **Live tmux testing session required:** After the implementation passes unit and E2E tests, run a real terminal tmux session that exercises the changed behavior. The task is not done until the live session succeeds.
## Completion Validation

- [x] **Unit tests** — `cargo test --lib` covers the changed logic and all new/modified unit tests pass.
- [x] **E2E tests** — `cargo test --workspace` passes (1 pre-existing flaky test: `tests::slash::session::resume_loads_most_recent_session` fails in full suite but passes in isolation — unrelated to this change).
- [x] **Live tmux run tests** — the change is exercised in a real terminal tmux session (or a live CLI/headless scenario if the task does not affect the TUI).

## Implementation

- `actor.rs:start_with_bus` now captures `coordinator_join` and `tcp_join` from `tokio::spawn` calls
- `SpawnedHandles` gains `coordinator_join: JoinHandle` and `tcp_join: Option<JoinHandle>` fields
- `LeaderHandle` stores both handles; `shutdown()` collects all joins and awaits them in parallel with a 5-second timeout
- `Clone` impl sets `coordinator_join: None` (handles cannot be cloned; only the original handle awaits)
