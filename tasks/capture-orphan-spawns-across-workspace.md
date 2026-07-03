# Capture orphan `tokio::spawn` calls across the workspace

**Status**: todo
**Milestone**: R7
**Category**: Architecture / Actors
**Priority**: P0

**Depends on**: fix-build-rs-lint-scope-and-tests
**Blocks**: none

## Description

Multiple production files outside `runie-core` spawn tasks and immediately drop the `JoinHandle`, violating the SSOT ADR rule that "Every spawned task has an owner." Known locations:

- `crates/runie-tui/src/bootstrap.rs:466, 474, 529, 547, 561, 562`
- `crates/runie-tui/src/ui_actor/mod.rs:282`
- `crates/runie-tui/src/ui_actor/effects.rs:29, 34`
- `crates/runie-cli/src/server.rs:46`
- `crates/runie-core/src/shell.rs:199`
- `crates/runie-agent/src/subagent.rs:196`

## Acceptance Criteria

- [ ] Every `tokio::spawn` in the listed files captures its `JoinHandle`.
- [ ] Captured handles are owned by a struct (`TuiRuntime`, `ServerRuntime`, etc.) or a `JoinSet` that is awaited on shutdown.
- [ ] Panics and unexpected exits in these tasks become observable.
- [ ] `cargo test --workspace` passes.
- [ ] `cargo check --workspace` passes with no new warnings.

## Tests

### Layer 1 — State/Logic
- [ ] `runtime_stores_all_task_handles` — a `TuiRuntime`/`ServerRuntime` struct holds handles for every spawned daemon.

### Layer 2 — Event Handling
- [ ] `shutdown_awaits_all_tasks` — quitting the app joins every handle within a bounded timeout.

### Layer 3 — Rendering
- [ ] N/A — no rendering change.

### Layer 4 — Provider Replay / Mock-Tool E2E
- [ ] `headless_shutdown_joins_tasks` — headless run completes with no orphan tasks.

### Live Tmux Testing Session
- [ ] Start and quit the TUI repeatedly; verify no leaked tasks via logs or process inspection.

## Files touched

- `crates/runie-tui/src/bootstrap.rs`
- `crates/runie-tui/src/ui_actor/mod.rs`
- `crates/runie-tui/src/ui_actor/effects.rs`
- `crates/runie-cli/src/server.rs`
- `crates/runie-core/src/shell.rs`
- `crates/runie-agent/src/subagent.rs`

## Notes

- Supersedes the remaining work from `enforce-observed-async-work-in-all-actors.md`.
- `LeaderHandle::shutdown` may serve as a pattern for awaiting actor-owned tasks.
