# Session Replay Startup Ordering

**Status**: done
**Milestone**: R3
**Category**: Sessions
**Priority**: P0

**Depends on**: `event-bus-replay-semantics`
**Blocks**: none

## Description

`SessionActor::run()` calls `replay_existing_events()` (publishing to the bus) before it subscribes, and `main.rs` creates `UiActor` with `bus.subscribe()` (no replay) *after* spawning `SessionActor`. In production, replayed durable events are broadcast while no subscriber is listening, so resuming a session does not restore prior messages in the UI. The existing test passes only because the test subscriber is created before the actor is spawned.

## Acceptance Criteria

- [ ] The UI actor is subscribed with replay before `SessionActor` publishes durable replay events.
- [ ] Resuming a session restores prior messages in the TUI.
- [ ] Existing session actor tests still pass.
- [ ] `cargo test --workspace` succeeds.

## Tests

### Layer 1 — State/Logic
- [ ] `session_actor_replays_after_subscriber_ready` — spawn subscriber first, then start actor, assert replay received.

### Layer 2 — Event Handling
- [ ] `ui_actor_receives_replay_on_resume` — TUI startup sequence restores messages.

### Layer 3 — Rendering
- [ ] `resumed_session_renders_prior_messages` — TestBackend shows prior user/assistant messages after resume.

### Layer 4 — Smoke / Crash
- [ ] `smoke_resume_session_shows_history` — tmux session resume shows history.

## Files touched

- `crates/runie-core/src/session_actor.rs`
- `crates/runie-tui/src/main.rs`

## Notes

Two valid fixes: (a) spawn `UiActor` first with `subscribe_with_replay`, then start `SessionActor`; or (b) have `SessionActor` subscribe before replaying and emit replay after confirming a subscriber is ready. Option (a) is simpler and less invasive.
