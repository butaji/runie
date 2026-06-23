# ViewActor owns ViewState

**Status**: todo
**Milestone**: R4
**Category**: TUI / Rendering
**Priority**: P0

**Depends on**: actor-owned-state-ssot, event-taxonomy-for-actor-state-sync, app-state-read-only-projection, session-actor-owns-session-state, input-actor-owns-input-state
**Blocks**: none

## Description

`ViewState` and the derived feed cache (`elements_cache`, `posts`, `line_counts`, `total_lines`, etc.) are written from dozens of places. `mark_dirty()` and `messages_changed()` are universal side-effects. Create a `ViewActor` that owns all view/cache state and is updated only by facts from other actors.

Current violators:
- `model/state/app_state.rs` — `mark_dirty`, `messages_changed`, `set_last_visible_height`, `set_last_content_width`.
- `model/cache.rs` — `ensure_fresh`, `tick_animation`, `animate_tokens`, `update_speed`, `clear_expired_transient`.
- `update/input/scroll.rs` — scroll and selected post.
- `update/input/mod.rs` — mouse position, vim nav, escape.
- `update/input/nav.rs` — vim nav mode.
- `update/system.rs` — `ToggleVimMode`, `NewSession` set `input_receiver` / `cached_settings_valid`.
- `update/session.rs` — queue delivery sets scroll.
- `update/dispatch.rs` — IO events set scroll/vim nav.
- `update/dialog/*.rs` — open/close/router/panel/toggle/form set `input_receiver`, `cached_settings_valid`, `cached_session_tree_valid`.
- `update/login_flow.rs` — close/cancel sets `input_receiver`, `cached_auth_valid`.
- `update/agent/core.rs` — `clear_turn_state` and `add_error` touch `vim_nav_pending`.
- `runie-tui/src/main.rs` — `init_terminal_state` sets dimensions.

## Acceptance criteria

- [ ] `ViewActor` is an mpsc actor holding the authoritative `ViewState`.
- [ ] `ViewMsg` covers: `Invalidate`, `MessagesChanged`, `Scroll { direction }`, `PageUp`, `PageDown`, `GoToTop`, `GoToBottom`, `ElementJump { direction }`, `MouseMoved { row, col }`, `MouseClicked { ... }`, `TerminalSized { width, height }`, `DialogOpened`, `DialogClosed`, `VimNav { enabled, selected_post }`, `ToggleExpandAll`, `TurnEnded`, `TurnErrored`, `AnimationTick`.
- [ ] `AppState.view` is private; reads go through an immutable accessor.
- [ ] `mark_dirty()` and `messages_changed()` helpers are removed from `AppState`.
- [ ] `ensure_fresh` (feed cache rebuild) and `tick_animation` are internal `ViewActor` helpers triggered by `ViewMsg::MessagesChanged` / `ViewMsg::AnimationTick`.
- [ ] `input_receiver` management is centralized in `ViewActor`: dialog openers/close send `ViewMsg::DialogOpened`/`DialogClosed`.
- [ ] Terminal resize sends `ViewMsg::TerminalSized` from both TUI init and input resize handler.
- [ ] `cargo test --workspace` passes.

## Tests

### Layer 1 — State/Logic
- [ ] `view_actor_invalidate_sets_dirty` — `Invalidate` sets `dirty=true` and emits `ViewChanged`.
- [ ] `view_actor_messages_changed_rebuilds_cache` — `MessagesChanged` runs `ensure_fresh` and updates `message_gen`.
- [ ] `view_actor_terminal_sized_updates_dimensions` — resize updates `last_content_width/height`.

### Layer 2 — Event Handling
- [ ] `dialog_open_sends_dialog_opened` — opening any dialog routes `DialogOpened` to `ViewActor`.
- [ ] `scroll_event_routes_to_view_actor` — scroll keys send `ViewMsg::Scroll`.

### Layer 3 — Rendering
- [ ] `view_actor_invalidated_triggers_render` — `Event::ViewChanged` causes `RenderActor` to draw.
- [ ] `feed_cache_rebuilds_after_messages_changed` — a `TestBackend` render after a message append shows the new message.

### Layer 4 — Provider Replay / Mock-Tool E2E
- [ ] `streaming_turn_view_stays_consistent` — a streaming mock-provider turn updates the feed without direct `view.*` writes.

## Files touched

- `crates/runie-core/src/actors/view/` — new `mod.rs`, `messages.rs`, `actor.rs`.
- `crates/runie-core/src/model/state/app_state.rs` — private `view`, remove `mark_dirty`/`messages_changed`.
- `crates/runie-core/src/model/cache.rs` — move cache logic into `ViewActor`; keep pure cache builders.
- `crates/runie-core/src/update/input/scroll.rs` — emit `ViewMsg`.
- `crates/runie-core/src/update/input/mod.rs` — mouse/escape/resize emit `ViewMsg`.
- `crates/runie-core/src/update/input/nav.rs` — vim nav emit `ViewMsg`.
- `crates/runie-core/src/update/system.rs` — `ToggleVimMode`/`NewSession` emit `ViewMsg`.
- `crates/runie-core/src/update/session.rs` — queue scroll emit `ViewMsg`.
- `crates/runie-core/src/update/dispatch.rs` — IO scroll emit `ViewMsg`.
- `crates/runie-core/src/update/dialog/*.rs` — dialog focus emit `ViewMsg`.
- `crates/runie-core/src/update/login_flow.rs` — login close emit `ViewMsg`.
- `crates/runie-core/src/update/agent/core.rs` — turn end/error emit `ViewMsg`.
- `crates/runie-tui/src/main.rs` — init terminal state emits `ViewMsg::TerminalSized`.

## Notes

- `ViewActor` is the natural consumer of `SessionChanged` and `InputChanged` facts; it decides when to rebuild caches and invalidate rendering.
- Keep pure cache-building helpers in `model/cache.rs` so `ViewActor` stays a thin scheduler.
