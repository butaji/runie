# Unify Session Persistence

**Status**: todo
**Milestone**: R3
**Category**: Sessions
**Priority**: P0

**Depends on**: fix-durable-event-mapping
**Blocks**: (none)

## Description

Two session persistence models exist: monolithic JSON `Session` for `/save`/`/load` and per-session `redb` durable events in `SessionActor`. There is no unified replay path and the stores can diverge.

## Acceptance Criteria

- [ ] `SessionStore`/`DurableCoreEvent` becomes the single source of truth.
- [ ] `/save` and `/load` replay durable events instead of reading a monolithic JSON file.
- [ ] Legacy JSON sessions migrate or remain read-only with a clear deprecation path.
- [ ] `cargo test --workspace` succeeds.

## Tests

### Layer 1 — State/Logic
- [ ] `save_replays_events` — `/save` persists durable events.
- [ ] `load_replays_events` — `/load` restores state by replaying.

### Layer 2 — Event Handling
- [ ] `session_actor_replays_to_uactor` — replayed events feed the UI projection.

## Files touched

- `crates/runie-core/src/session.rs`
- `crates/runie-core/src/session_store.rs`
- `crates/runie-core/src/session_actor.rs`
- `crates/runie-core/src/commands/handlers/session.rs`

## Notes

Coordinate with `adopt-redb-session-store.md` and `fix-session-store-atomic-writes.md`.
