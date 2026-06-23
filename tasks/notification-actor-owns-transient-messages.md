# NotificationActor owns transient messages

**Status**: todo
**Milestone**: R4
**Category**: Core / State
**Priority**: P1

**Depends on**: actor-owned-state-ssot, event-taxonomy-for-actor-state-sync, app-state-read-only-projection
**Blocks**: none

## Description

Transient notification state (`transient_message`, `transient_until`, `transient_level`) is mutated in at least six files, and expiration is currently triggered from the render-path snapshot builder. Create a `NotificationActor` that owns the timer and the state.

Current violators:
- `model/state/app_state.rs` — initializes transient fields.
- `update/system.rs` — `set_transient`, `clear_transient`, `add_system_msg`.
- `notification.rs` — `Notification::show`, `notification::dismiss`.
- `model/cache.rs` — `clear_expired_transient` called during snapshot build.
- `update/dispatch.rs` — session-store and IO handlers call `state.notify(...)` / `state.add_system_msg(...)`.
- `update/dialog/router.rs` — `CommandResult::Warning` calls `state.notify(...)`.

## Acceptance criteria

- [ ] `NotificationActor` is an mpsc actor owning `transient_message`, `transient_until`, `transient_level`.
- [ ] `NotificationMsg` covers: `Show { content, level, duration }`, `ShowError { content }`, `Dismiss`, `SystemMessage { content }`.
- [ ] `AppState.transient_*` fields are private; reads go through an immutable accessor.
- [ ] `NotificationActor` emits `Event::TransientMessage` / `Event::TransientError` / `Event::ClearTransient`.
- [ ] `set_transient` / `clear_transient` / `add_system_msg` helpers are removed from `AppState` and `update/system.rs`.
- [ ] Expiration is handled by the actor on a periodic tick or a `tokio::time::sleep` task, not in `model/cache.rs`.
- [ ] Session-store/IO handlers and dialog router send `NotificationMsg` instead of calling `state.notify`.
- [ ] `cargo test --workspace` passes.

## Tests

### Layer 1 — State/Logic
- [ ] `notification_actor_show_sets_timer` — `Show` sets `transient_until` based on duration.
- [ ] `notification_actor_expired_dismisses` — after timeout the actor emits `ClearTransient`.

### Layer 2 — Event Handling
- [ ] `session_saved_shows_notification` — `SessionSaved` event sends `NotificationMsg::Show`.
- [ ] `command_warning_shows_notification` — `CommandResult::Warning` sends `NotificationMsg::Show`.

### Layer 3 — Rendering
- [ ] `transient_message_renders_then_clears` — notification appears and disappears in `TestBackend`.

### Layer 4 — Provider Replay / Mock-Tool E2E
- [ ] N/A.

## Files touched

- `crates/runie-core/src/actors/notification/` — new `mod.rs`, `messages.rs`, `actor.rs`.
- `crates/runie-core/src/model/state/app_state.rs` — private transient fields.
- `crates/runie-core/src/update/system.rs` — remove `set_transient`/`clear_transient`/`add_system_msg`.
- `crates/runie-core/src/notification.rs` — delegate to `NotificationActor`.
- `crates/runie-core/src/model/cache.rs` — remove `clear_expired_transient`; consume `ClearTransient` events.
- `crates/runie-core/src/update/dispatch.rs` — session/IO handlers emit `NotificationMsg`.
- `crates/runie-core/src/update/dialog/router.rs` — warning result emits `NotificationMsg`.

## Notes

- `SystemMessage` notifications that append to `session.messages` should be split: the notification text goes to `NotificationActor`, while the message append goes to `SessionActor`.
