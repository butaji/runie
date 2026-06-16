# Unify Session Persistence

**Status**: done
**Milestone**: R3
**Category**: Sessions
**Priority**: P0

**Depends on**: fix-durable-event-mapping
**Blocks**: (none)

## Description

Two session persistence models exist: monolithic JSON `Session` for `/save`/`/load` and per-session `redb` durable events in `SessionActor`. There is no unified replay path and the stores can diverge.

## Acceptance Criteria

- [x] `SessionStore`/`DurableCoreEvent` becomes the single source of truth.
- [x] `/save` and `/load` replay durable events instead of reading a monolithic JSON file.
- [x] Legacy JSON sessions remain read-only with a clear deprecation path.
- [x] `cargo test --workspace` succeeds.

## Tests

### Layer 1 — State/Logic
- [x] `save_replays_events` — `/save` persists durable events.
- [x] `load_replays_events` — `/load` restores state by replaying.

### Layer 2 — Event Handling
- [x] `session_actor_replays_to_uactor` — replayed events feed the UI projection.

## Files touched

- `crates/runie-core/src/session.rs`
- `crates/runie-core/src/session_store.rs`
- `crates/runie-core/src/session_actor.rs`
- `crates/runie-core/src/session_replay.rs`
- `crates/runie-core/src/event/durable.rs`
- `crates/runie-core/src/event/variants.rs`
- `crates/runie-core/src/update/mod.rs`
- `crates/runie-core/src/update/session.rs`
- `crates/runie-core/src/commands/dsl/handlers/session/io.rs`
- `crates/runie-core/src/commands/dsl/handlers/session/mod.rs`

## Notes

- `SessionStore` (redb) is now the single source of truth for `/save`/`/load`.
- Durable events are replayed into `AppState` via `session_replay::replay_events`.
- `SessionActor` replays existing events onto the event bus on startup.
- Legacy JSON sessions remain readable through `/import` and as a fallback in `/load`; `session.rs` is marked deprecated.
- Coordinates with `adopt-redb-session-store.md` and `fix-session-store-atomic-writes.md`.
