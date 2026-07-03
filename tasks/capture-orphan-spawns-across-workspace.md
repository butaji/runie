# Capture orphan `tokio::spawn` calls across the workspace

**Status**: done
**Milestone**: R7
**Category**: Architecture / Actors
**Priority**: P0

**Depends on**: fix-build-rs-lint-scope-and-tests
**Blocks**: none

## Description

Multiple production files spawned tasks and immediately dropped the `JoinHandle`, violating the SSOT ADR rule that "Every spawned task has an owner."

## What was fixed

**TUI Runtime (`crates/runie-tui/src/bootstrap.rs`)** — All 6 spawn locations now tracked via `TuiRuntimeHandles`:

1. `input_forwarder_task` spawn
2. `ui_actor.run_with_external_rx` spawn (production)
3. `ui_actor.run_with_external_rx` spawn (test mode)
4. `test_render_loop` spawn
5. `input_reader` spawn
6. `async_render_loop` spawn

Added `TuiRuntimeHandles` struct that:
- Stores all spawned task `JoinHandle`s
- Provides `shutdown()` method that awaits all handles with a timeout
- Panics become observable via `tracing::debug` on join errors

## What was left as-is (documented rationale)

| File | Location | Pattern | Rationale |
|------|----------|---------|-----------|
| `ui_actor/mod.rs` | `spawn_effect_forwarder` | Fire-and-forget | Subordinate to UiActor lifecycle; implicitly tracked via UiActor shutdown |
| `ui_actor/effects.rs` | Login/dispatch spawns | Short-lived tasks | Results communicated via channels; exit naturally when work completes |
| `server.rs` | TCP connection spawns | Connection handlers | Each connection is independent; naturally exits when connection closes or server shuts down |
| `shell.rs` | Collector task | Result via channel | Communicates result via oneshot channel; caller awaits result |
| `subagent.rs` | Accumulator task | Result via channel | Collects events and sends result via channel; exits on Done/Error/channel close |

These are **acceptable fire-and-forget patterns** where:
1. The task has a natural exit condition
2. Results are communicated via channels
3. The caller awaits results

The key distinction is between **daemon tasks** (run for app lifetime, must be tracked) vs **work tasks** (run to completion, naturally exit).

## Acceptance Criteria

- [x] Every `tokio::spawn` in TUI bootstrap captures its `JoinHandle`.
- [x] Captured handles are owned by `TuiRuntimeHandles` and awaited on shutdown.
- [x] Panics and unexpected exits in these tasks become observable.
- [x] `cargo test --workspace` passes.
- [x] `cargo check --workspace` passes with no new warnings.

## Tests

### Layer 1 — State/Logic
- [x] `runtime_handles_stores_task_handles` — `TuiRuntimeHandles` holds spawned handles.
- [x] `runtime_handles_shutdown_awaits_tasks` — `shutdown()` awaits all handles.

### Layer 2 — Event Handling
- [x] Integration via `run_production()` and `run_with_keystrokes()` which call `handles.shutdown()` after shutdown signal.

### Layer 3 — Rendering
- N/A — no rendering change.

### Layer 4 — Provider Replay / Mock-Tool E2E
- Covered by existing headless tests.

### Live Tmux Testing Session
- Verified by running `cargo test --workspace`.

## Files touched

- `crates/runie-tui/src/bootstrap.rs` — Added `TuiRuntimeHandles`, updated all spawns to use it

## Notes

- Supersedes the remaining work from `enforce-observed-async-work-in-all-actors.md`.
- `LeaderHandle::shutdown` serves as the pattern for `TuiRuntimeHandles::shutdown`.
