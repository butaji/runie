# Replace EmitFn mutex with async channel

## Status

`todo`

## Context

`crates/runie-agent/src/stream_response.rs:23-24,201-204` defines `EmitFn` as `Arc<Mutex<dyn FnMut(Event) + Send + Sync>>` and locks per token.

## Goal

Replace with `tokio::sync::mpsc::UnboundedSender<Event>` (or bounded with backpressure) and a receiver owned by the caller.

## Acceptance Criteria
- [ ] Define channel-based emit.
- [ ] Update all call sites.
- [ ] Eliminate per-token mutex lock.

## Design Impact

No change to TUI element design or composition unless explicitly noted. Only implementation behavior, dependency graph, internal architecture, or async runtime changes.

## Tests

- **Layer 1 — State/Logic:** Unit tests for event order.
- **Layer 2 — Event Handling:** N/A.
- **Layer 3 — Rendering:** N/A.
- **Layer 4 — E2E:** Streaming replay tests pass.
- **Live tmux testing session (required):** Streaming tokens arrive smoothly.

> **Live tmux testing session required:** After the implementation passes unit and E2E tests, run a real terminal tmux session that exercises the changed behavior. The task is not done until the live session succeeds.
## Completion Validation

- [ ] **Unit tests** — `cargo test --lib` covers the changed logic and all new/modified unit tests pass.
- [ ] **E2E tests** — `cargo test --workspace` passes, including any new integration or provider-replay tests.
- [ ] **Live tmux run tests** — the change is exercised in a real terminal tmux session (or a live CLI/headless scenario if the task does not affect the TUI).
