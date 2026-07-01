# Track and cancel in-flight agent turn JoinHandle

## Status

`todo`

## Context

`crates/runie-agent/src/actor.rs:137-164` spawns the agent turn as a detached `tokio::task` and discards the `JoinHandle`. A second `Run` while one is in flight leaves the previous turn running with no cancellation or await path.

## Goal

Store the `JoinHandle` in `AgentActorState`, reject or queue overlapping `Run`s, and await/cancel it on `Abort`/shutdown.

## Acceptance Criteria
- [ ] Store `Option<JoinHandle<()>>` in actor state.
- [ ] Reject or queue a new `Run` while one is in flight.
- [ ] Cancel/await the handle on `Abort` and actor shutdown.
- [ ] Surface turn errors instead of dropping them.

## Design Impact

No change to TUI element design or composition unless explicitly noted. Only implementation behavior, dependency graph, internal architecture, or async runtime changes.

## Tests

- **Layer 1 — State/Logic:** Unit tests for overlap rejection and abort cancellation.
- **Layer 2 — Event Handling:** Actor handles `Abort` fact while turn running.
- **Layer 3 — Rendering:** N/A.
- **Layer 4 — E2E:** Provider replay with overlapping runs behaves deterministically.
- **Live tmux validation:** Submit a second message before first turn ends; UI stays consistent.

## Completion Validation

- [ ] **Unit tests** — `cargo test --lib` covers the changed logic and all new/modified unit tests pass.
- [ ] **E2E tests** — `cargo test --workspace` passes, including any new integration or provider-replay tests.
- [ ] **Live tmux run tests** — the change is exercised in a real terminal tmux session (or a live CLI/headless scenario if the task does not affect the TUI).
