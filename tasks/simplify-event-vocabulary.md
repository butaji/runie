# Simplify the event vocabulary

**Status**: done  
**Milestone**: R4  
**Category**: Core / State  
**Priority**: P0  

**Depends on**: remove-io-from-runie-core  
**Blocks**: none  

## Description

The `Event` enum has been nested into typed sub-enums (`Input`, `Agent`, `Control`, `Dialog`, `System`, `Config`, `Session`, `Io`, plus top-level `ConfigLoaded`). The flat `DurableCoreEvent` used for session persistence is unchanged, preserving serde compatibility with existing session stores. Dispatchers and helper functions were updated to operate on the typed sub-enums where feasible.

## Acceptance Criteria

- [x] `Event` is a nested enum with sub-enums: `Input`, `Agent`, `Control`, `Dialog`, `System`, `Config`, `Session`, `Io`, plus `ConfigLoaded`.
- [x] `update/dispatch.rs` categorizers are deleted.
- [x] `AppState::update` matches every `Event` variant and delegates to one handler per category.
- [x] Cross-cutting rules like `ensure_turn_complete_last` are encapsulated in the state method that appends assistant content.
- [x] Duplicate events (`TrustChanged`/`TrustSet`, `TransientMessage`/`TransientError`) are merged where semantics overlap.
- [x] Serde compatibility with existing session stores is preserved; `DurableCoreEvent` remains flat and unchanged.
- [x] `cargo test --workspace` succeeds.

## Tests

### Layer 1 — State/Logic
- [x] `event_dispatch_routes_each_subenum` — every sub-enum has a handler branch.
- [x] `assistant_response_appends_turn_complete_automatically` — `TurnComplete` is emitted as a side effect of the append helper, not in every agent branch.
- [x] `trust_set_event_is_idempotent` — only one trust event variant exists.

### Layer 2 — Event Handling
- [x] `nested_events_serialize_round_trip` — events still serialize to the same session format.
- [x] `dialog_event_routes_to_dialog_handler` — `Event::Dialog(...)` reaches the dialog module.

### Layer 3 — Rendering
- [ ] N/A — event shape change, not rendering.

### Layer 4 — Smoke / Crash
- [x] `smoke_session_replay_after_event_nesting` — a saved session replays correctly after the enum change.

## Files touched

- `crates/runie-core/src/event/variants.rs`
- `crates/runie-core/src/event/mod.rs`
- `crates/runie-core/src/update/mod.rs`
- `crates/runie-core/src/update/dispatch.rs`
- `crates/runie-core/src/update/agent/mod.rs`
- `crates/runie-core/src/update/dialog/mod.rs`
- `crates/runie-core/src/update/system.rs`
- `tasks/simplify-event-vocabulary.md`

## Phase 3a — Merge duplicate events and encapsulate cross-cutting rules

- [x] Merge `TrustChanged`/`TrustSet` into a single `TrustSet` event.
- [x] Merge `TransientMessage`/`TransientError` into `TransientMessage { content, level }`.
- [x] Encapsulate `ensure_turn_complete_last` in the assistant append helpers.
- [x] Update tests and guardrail baseline for the reduced variant count.

## Phase 3b — Collapse dispatch into category handlers

- [x] `AppState::update` uses a single `match event` that routes to one handler per category (`Input`, `Agent`, `Control`, `Dialog`, `System`, `Config`, `Session`, `Io`).
- [x] Early-return special cases (`try_handle_dialog_event_input`, `try_handle_vim_dialog_back_input`, `try_handle_vim_nav_event_input`, `ConfigLoaded`) moved into the relevant category handler or handled first.
- [x] `update/dispatch.rs` categorizers (`EventCategory`, `is_*_event`, `dispatch_event`) deleted.
- [x] Dialog login flow / providers routing moved into `update/dialog/mod.rs` helpers.
- [x] Persistence, session store, bootstrap, and IO events moved into their category handlers (`config.rs`, `session.rs`, `system.rs`, `io.rs`).
- [x] Tests updated; new Layer 1 test `event_dispatch_routes_each_category` added.

## Notes

- Preserve the existing serde tag/content shape for durable events so old session files continue to load.
- Type aliases can temporarily keep old names (`pub type InputEvent = event::InputEvent`) to reduce diff size.
- The event variant budget test from Phase 0 must pass after nesting; the baseline will be lowered.
