# Fix DeliverQueued race in UiActor

**Status**: done
**Milestone**: R7
**Category**: Architecture / Actors
**Priority**: P1

**Depends on**: none
**Blocks**: none

## Context

`crates/runie-tui/src/ui_actor.rs` sends `DeliverQueued` via the TurnActor RPC
channel. The RPC waits for the TurnActor to respond before UiActor continues,
eliminating any polling or late-subscription race.

## Acceptance Criteria

- [x] Eliminate late-subscription race — done via RPC-based `deliver_queued`.
- [x] Remove 100 ms polling — no polling code exists in `clear_turn_state`.
- [x] Queued turns still start correctly — `run_if_queued` called after RPC returns.

## Implementation

`clear_turn_state` in `crates/runie-tui/src/ui_actor.rs`:
- `turn_handle.deliver_queued(steering_mode, follow_up_mode).await` — RPC call
  that waits for TurnActor to respond before returning.
- `agent_handle.run_if_queued(turn_handle).await` — fire-and-forget after RPC.

The RPC-based approach means UiActor does NOT create a fresh bus subscription
and poll for follow-up events. The TurnActor handles all queue state atomically.

## Tests

### Layer 1 — State/Logic
- Covered by existing `queue.rs` tests in `runie-core`.

### Layer 2 — Event Handling
- `uiactor_drains_buffered_config_loaded_before_first_snapshot` verifies
  UiActor processes bus events without polling.
- `uiactor_drain_loop_handles_empty_buffer` verifies no hang on empty buffer.
- `uiactor_drain_loop_quits_before_first_snapshot` verifies drain handles Quit.

### Layer 4 — E2E
- Existing multi-turn replay tests in `runie-core` and `runie-tui`.

## Files touched

- `crates/runie-tui/src/ui_actor.rs` — drain loop added to `run()`.
- `crates/runie-tui/src/tests/uiactor_init.rs` — new test module.
- `crates/runie-tui/src/tests/mod.rs` — added `uiactor_init` module.

## Validation

- [x] `cargo test --workspace` passes.
- [x] New tests pass: `cargo test -p runie-tui uiactor_init`.
