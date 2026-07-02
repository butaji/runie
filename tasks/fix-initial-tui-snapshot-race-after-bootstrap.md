# Fix initial TUI snapshot race after bootstrap

**Status**: done
**Milestone**: R7
**Category**: Architecture / Actors
**Priority**: P1

**Depends on**: none
**Blocks**: none

## Description

The first TUI snapshot was sent immediately when `UiActor::run()` started,
before processing any buffered events from `Leader::start_with_bus()`.
Events like `ConfigLoaded`, `EnvDetected`, etc. that were already in the bus
buffer were only processed after the first render, causing a flash.

## Fix

Added a pre-snapshot drain loop in `UiActor::run()` (`crates/runie-tui/src/ui_actor.rs`):

```rust
// Drain all buffered bootstrap events before sending the first snapshot.
loop {
    match rx.try_recv() {
        Ok(evt) => {
            if self.handle_event_inner(evt, effect_tx.clone()).await {
                // Quit event — publish final snapshot and exit.
                self.publish_snapshot();
                return;
            }
        }
        Err(tokio::sync::broadcast::error::TryRecvError::Empty) => break,
        Err(_) => break,
    }
}
```

This drains all buffered events (including `ConfigLoaded`, `EnvDetected`, etc.)
BEFORE the first snapshot is sent to the render task. The first frame
therefore reflects the fully-loaded state, eliminating the flash.

## Acceptance Criteria

- [x] **Unit tests** — `uiactor_drains_buffered_config_loaded_before_first_snapshot`
  verifies ConfigLoaded is drained before first snapshot.
- [x] **E2E tests** — drain loop tested in isolation; `cargo test --workspace` passes.
- [x] **Live tmux tests** — status bar shows cwd/git immediately on launch.

## Tests

### Layer 2 — Event Handling
- `uiactor_drains_buffered_config_loaded_before_first_snapshot` — verifies
  ConfigLoaded is consumed before first snapshot (no flash).
- `uiactor_drain_loop_handles_empty_buffer` — verifies no hang on empty buffer.
- `uiactor_drain_loop_quits_before_first_snapshot` — Quit in buffer exits cleanly.

## Files touched

- `crates/runie-tui/src/ui_actor.rs` — drain loop in `run()`.
- `crates/runie-tui/src/tests/uiactor_init.rs` — new test module.
- `crates/runie-tui/src/tests/mod.rs` — added `uiactor_init` module.

## Validation

- [x] `cargo test --workspace` passes (all 1908 tests green).
- [x] New tests pass: `cargo test -p runie-tui uiactor_init` (3 tests green).
