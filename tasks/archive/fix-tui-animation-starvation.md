# Remove biased select and speed up animation tick in UiActor

**Status**: done
**Milestone**: R4
**Category**: TUI / Rendering
**Priority**: P1
**Depends on**: none
**Blocks**: none
**Completed in**: current

## Description

`UiActor::run` used `tokio::select! { biased; Ok(evt) = rx.recv() => ... _ = anim.tick() => ... }`. During streaming bursts the event branch is always ready, starving the animation timer so the spinner/status timer freezes. The timer interval was also 200 ms, which looks jerky.

## Acceptance Criteria

- [x] Remove `biased` from the `tokio::select!` so the animation timer gets fair scheduling.
- [x] Reduce `ANIM_MS` from 200 ms to 100 ms for smoother spinner/status updates.
- [x] `cargo check -p runie-tui` succeeds.
- [x] `cargo test -p runie-tui` succeeds.

## Tests

- [x] Layer 2 Event Handling: existing `ui_actor_updates_state_from_bus_event` test still passes.
- [x] Layer 4 Smoke: `cargo test -p runie-tui` passes.

## Files touched

- `crates/runie-tui/src/ui_actor.rs`

## Notes

Minimal change; no behavioral differences beyond avoiding blocking the async runtime.
