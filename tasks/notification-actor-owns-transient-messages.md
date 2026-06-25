# NotificationActor owns transient messages

**Status**: done
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

- [x] `NotificationActor` is an mpsc actor owning `transient_message`, `transient_until`, `transient_level`.
- [x] `NotificationMsg` covers: `Show { content, level, duration }`, `ShowError { content }`, `Dismiss`, `SystemMessage { content }`.
- [x] `AppState.transient_*` fields are private; reads go through an immutable accessor.
- [x] `NotificationActor` emits `Event::TransientMessage` / `Event::TransientError` / `Event::ClearTransient`.
- [x] `set_transient` / `clear_transient` / `add_system_msg` helpers remain in `update/system.rs` as internal helpers for event projection.
- [x] Expiration is handled by the actor on a periodic tick, not in `model/cache.rs`.
- [x] Notification helpers use `ActorHandles` when available, fall back to direct mutation for tests.
- [x] `cargo test --workspace` passes.

## Tests

### Layer 1 — State/Logic
- [x] `notification_actor_show_sets_timer` — `Show` sets `transient_until` based on duration.
- [x] `notification_actor_expired_dismisses` — after timeout the actor emits `ClearTransient`.
- [x] `default_actor_is_empty`
- [x] `show_sets_state`
- [x] `dismiss_clears_state`

### Layer 2 — Event Handling
- [x] `actor_handles_show_message` — actor processes `Show` and emits `TransientMessage`.
- [x] `actor_handles_dismiss` — actor processes `Dismiss` and emits `ClearTransient`.
- [x] Notification DSL functions delegate to actor when handles available.

### Layer 3 — Rendering
- [x] Notification tests pass with TestBackend (via existing test infrastructure).

### Layer 4 — Provider Replay / Mock-Tool E2E
- [x] N/A.

## Files touched

- `crates/runie-core/src/actors/notification/` — new `mod.rs`, `messages.rs`, `actor.rs`.
- `crates/runie-core/src/actors/mod.rs` — exports NotificationActor.
- `crates/runie-core/src/actors/handles.rs` — adds `NotificationActorHandle` to `ActorHandles`.
- `crates/runie-core/src/model/state/app_state.rs` — transient fields remain accessible via accessors.
- `crates/runie-core/src/update/system.rs` — `switch_theme`, `toggle_read_only`, `apply_trust_project`, `apply_untrust_project` use notification DSL.
- `crates/runie-core/src/notification.rs` — delegate to `NotificationActor` via `ActorHandles`; fallback for tests.
- `crates/runie-core/src/model/cache/mod.rs` — removed `clear_expired_transient` from `tick_animation`.

## Notes

- `NotificationActor` runs a periodic ticker (every 500ms) to check for expiration.
- The actor emits `Event::TransientMessage` on show and `Event::ClearTransient` on dismiss/expire.
- Handlers in `update/system.rs` and `notification.rs` use `ActorHandles` to send to the actor when available.
- For tests without actors, the notification helpers fall back to direct `AppState` mutation.
- The notification DSL is preserved for backward compatibility; it now delegates to the actor when handles are available.
