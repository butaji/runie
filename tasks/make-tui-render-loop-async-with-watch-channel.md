# Make TUI render loop async with watch channel

## Status

`todo`

## Context

`crates/runie-tui/src/main.rs:257-307` runs the render loop in a blocking `std::thread` fed by `std::sync::mpsc::sync_channel(1)` and polls `recv_timeout(FRAME_TIME)` every 16 ms.

## Goal

Make rendering an async tokio task waiting on `tokio::sync::watch::changed`; wrap only `terminal.draw` in `spawn_blocking` if needed.

## Acceptance Criteria
- [ ] Replace mpsc + polling with watch channel.
- [ ] Preserve 60 FPS update behavior.
- [ ] Graceful shutdown unchanged.

## Design Impact

No change to TUI element design or composition unless explicitly noted. Only implementation behavior, dependency graph, internal architecture, or async runtime changes.

## Tests

- **Layer 1 — State/Logic:** N/A.
- **Layer 2 — Event Handling:** N/A.
- **Layer 3 — Rendering:** Render snapshots unchanged.
- **Layer 4 — E2E:** N/A.
- **Live tmux testing session (required):** TUI starts and quits cleanly.

> **Live tmux testing session required:** After the implementation passes unit and E2E tests, run a real terminal tmux session that exercises the changed behavior. The task is not done until the live session succeeds.
## Completion Validation

- [ ] **Unit tests** — `cargo test --lib` covers the changed logic and all new/modified unit tests pass.
- [ ] **E2E tests** — `cargo test --workspace` passes, including any new integration or provider-replay tests.
- [ ] **Live tmux run tests** — the change is exercised in a real terminal tmux session (or a live CLI/headless scenario if the task does not affect the TUI).
