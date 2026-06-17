# Extract UiActor from TUI Main Loop

**Status**: done
**Completed**: 2026-06-16
**Notes**: Created `runie_tui::ui_actor::UiActor` that owns `AppState`, subscribes to `EventBus<Event>`, applies events, and publishes `Snapshot` via a `watch` channel to `render_task`. Main loop now only sets up terminal/actors and waits for a shutdown signal. Side-effects, agent spawns, and keybinding reloads are handled inside `UiActor`. Added 2 Layer 2/3 tests. cargo test --workspace passes.
**Milestone**: R3
**Category**: Architecture / Actors
**Priority**: P0

**Depends on**: extract-core-monolith
**Blocks**: (none)

## Description

`runie-tui/src/main.rs` owns `AppState` directly and calls `state.update(evt)` inline. The `EventBus`/`Actor` infrastructure exists only to feed `SessionActor`, creating two parallel event paths. The architecture diagram in `docs/SPEC.md` shows a `UiActor` that owns state.

## Acceptance Criteria

- [ ] A `UiActor` owns `AppState`, subscribes to `EventBus<Event>`, and is the sole state mutator.
- [ ] `UiActor` sends `Snapshot` to a thin `RenderActor` via a `watch` channel.
- [ ] The TUI main loop only reads input and forwards it to the bus; it does not mutate `AppState`.
- [ ] `cargo test --workspace` succeeds.

## Tests

### Layer 2 — Event Handling
- [ ] `ui_actor_updates_state` — an event published to the bus is reflected in the next snapshot.

### Layer 3 — Rendering
- [ ] `render_actor_draws_snapshot` — `RenderActor` draws the snapshot without state mutation.

### Layer 4 — Smoke
- [ ] `tmux_smoke_starts` — binary still runs and renders.

## Files touched

- `crates/runie-core/src/actor.rs`
- `crates/runie-core/src/orchestrator_actor.rs`
- `crates/runie-tui/src/main.rs`
- `crates/runie-tui/src/lib.rs`

## Notes

This may require flattening `OrchestratorEvent` into `Event` first (see `flatten-event-enum.md`).
