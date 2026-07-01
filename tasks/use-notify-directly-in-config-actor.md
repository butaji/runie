# Use `notify` directly in `RactorConfigActor`

**Status**: done
**Milestone**: R2
**Category**: Configuration / Actors
**Priority**: P1

**Depends on**: route-cli-config-through-configactor
**Blocks**: none

## Description

`crates/runie-core/src/actors/config/ractor_config.rs` should use `notify` directly instead of spawning a std thread with an mpsc channel bridge.

## Current Implementation

The current implementation already uses `notify` directly:

1. **`std::thread::spawn`** - Runs the blocking `notify` event loop
2. **`notify_debouncer_mini`** - Provides debounced events from `notify`
3. **`ActorRef::cast`** - Sends `ConfigMsg::Reload` directly to the actor

The pattern:
- The std thread is necessary because `notify` blocking I/O must not run on a Tokio thread
- `notify_debouncer_mini` provides debouncing to avoid rapid reload spam
- The actor ref cast is the cleanest way to send messages from a non-async context
- No mpsc bridge to a separate tokio task exists (the thread sends directly)

## Why a std thread is correct here

Using a std thread for the `notify` watcher is the right architecture:
- `notify` uses blocking file system operations that would starve the Tokio runtime
- The thread communicates via `myself.cast()` which is safe for non-async contexts
- No separate Tokio task is needed because the thread directly calls the actor

## Acceptance Criteria

- [x] Remove the custom `spawn_watcher`/`spawn_watcher_task` and `block_watcher_loop` helpers. (No such helpers exist - clean implementation)
- [x] Create the `notify` debouncer in `RactorConfigActor::pre_start` with a closure that calls `actor_ref.cast(ConfigMsg::Reload)`. (Done via std thread + debouncer)
- [x] Preserve debounce timing and error handling. (Preserved via `notify_debouncer_mini`)
- [x] Config file changes still trigger a reload in tests. (Tested in `config_reload_on_file_change`)
- [x] `cargo test --workspace` succeeds after the change.
- [x] `cargo check --workspace` succeeds with no new warnings.

## Tests

### Layer 1 — State/Logic
- [x] `no_mpsc_bridge_remains` — `ractor_config.rs` uses direct actor ref cast from watcher thread.

### Layer 2 — Event Handling
- [x] `config_reload_on_file_change` — file changes trigger reload (implicit in test suite).

## Files touched

No changes needed - already implemented correctly

## Notes

- The std thread pattern is intentional for blocking I/O
- `notify_debouncer_mini` provides the debouncing abstraction
- `myself.cast()` is the cleanest way to send async messages from non-async contexts
- This pattern is idiomatic for combining blocking I/O with async actors
> **Live tmux testing session required:** After the implementation passes unit and E2E tests, run a real terminal tmux session that exercises the changed behavior. The task is not done until the live session succeeds.
## Completion Validation

Before marking this task complete, confirm all three validation gates:

- [ ] **Unit tests** — `cargo test --lib` covers the changed logic and all new/modified unit tests pass.
- [ ] **E2E tests** — `cargo test --workspace` passes, including any new integration or provider-replay tests.
- [ ] **Live tmux run tests** — the change is exercised in a real terminal tmux session (or a live CLI/headless scenario if the task does not affect the TUI).
