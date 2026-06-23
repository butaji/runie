# Coalesce queued events in UiActor before publishing snapshots

**Status**: done
**Milestone**: R4
**Category**: TUI / Rendering
**Priority**: P1
**Depends on**: none
**Blocks**: none
**Completed in**: current

## Description

`UiActor::run` handled one event, published a snapshot, then awaited the next event. During response-streaming bursts this produces one snapshot per token and amplifies render load. Events that are already queued can be drained and applied in a batch, then a single snapshot is published for the whole burst.

## Acceptance Criteria

- [x] After receiving an event, drain all currently queued events from the replay receiver and apply them before publishing a snapshot.
- [x] Side-effect logic (persistence, agent run_if_queued) still runs once per event.
- [x] A single `Quit` event still shuts down the actor.
- [x] `cargo check -p runie-tui` succeeds.
- [x] `cargo test -p runie-tui` succeeds.

## Tests

- [x] Layer 2 Event Handling: `ui_actor_updates_state_from_bus_event` still passes.
- [x] Layer 2 Event Handling: `login_key_submit_triggers_validation_effect` still passes.
- [x] Layer 4 Smoke: `cargo test -p runie-tui` passes.

## Files touched

- `crates/runie-tui/src/ui_actor.rs`

## Notes

Per-event side effects are preserved; only snapshot publication is batched.
