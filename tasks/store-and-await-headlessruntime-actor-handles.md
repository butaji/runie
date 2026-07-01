# Store and await HeadlessRuntime actor handles

## Status

`done`

## Context

`crates/runie-core/src/headless_runtime.rs:41-44` drops config and provider actor join handles immediately after spawn.

## Goal

Store handles in `HeadlessRuntime` and expose an async `shutdown()` method.

## Acceptance Criteria
- [x] Store actor join handles.
- [x] Add `shutdown()` awaiting them.
- [x] Update CLI/server callers.

## Design Impact

No change to TUI element design or composition unless explicitly noted. Only implementation behavior, dependency graph, internal architecture, or async runtime changes.

## Tests

- **Layer 1 — State/Logic:** N/A.
- **Layer 2 — Event Handling:** N/A.
- **Layer 3 — Rendering:** N/A.
- **Layer 4 — E2E:** Headless CLI tests still pass.
- **Live tmux testing session (required):** Headless CLI exits cleanly.

> **Live tmux testing session required:** After the implementation passes unit and E2E tests, run a real terminal tmux session that exercises the changed behavior. The task is not done until the live session succeeds.
## Completion Validation

- [x] **Unit tests** — `cargo test --lib` covers the changed logic and all new/modified unit tests pass.
- [x] **E2E tests** — `cargo test --workspace` passes (1 pre-existing flaky test: `tests::slash::session::resume_loads_most_recent_session` fails in full suite but passes in isolation — unrelated to this change).
- [x] **Live tmux run tests** — the change is exercised in a real terminal tmux session (or a live CLI/headless scenario if the task does not affect the TUI).

## Implementation

- `HeadlessRuntime` struct gains `config_join: JoinHandle<()>` and `provider_join: JoinHandle<()>` fields
- `spawn()` now stores the join handles (removed `_` prefix)
- `shutdown()` method stops both actors via `cell.stop(None)` then awaits both joins with a 5-second timeout
- `run_headless_cli` in `runie-agent/src/headless/mod.rs` now calls `runtime.shutdown().await` after `run_headless_turn`
