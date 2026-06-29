# Delete dead actor handle wrappers

**Status**: todo
**Milestone**: R6
**Category**: Architecture / Actors
**Priority**: P1

**Depends on**: migrate-production-actors-to-ractor
**Blocks**: actually-collapse-actor-handles-to-typed-map

## Description

Several legacy handle wrappers survived the actor migration and are not used in production: `SessionActorHandle`, `PersistenceActorHandle`, `SessionStoreActorHandle`, `ConfigActorHandle`, `PermissionActorHandle`, `ProviderActorHandle::legacy_tx`, and `GenericActorHandle`. Delete them and use `ractor::ActorRef<Msg>` directly.

## Acceptance Criteria

- [ ] Identify all dead wrappers (no production callers).
- [ ] Delete them and their trait implementations.
- [ ] Update any tests that used them to use `ractor::ActorRef`.
- [ ] `cargo test --workspace` succeeds after the change.
- [ ] `cargo check --workspace` succeeds with no new warnings.

## Tests

### Layer 1 — State/Logic
- [ ] `dead_handle_wrappers_removed` — grep confirms no `SessionActorHandle` etc. remain.

### Layer 2 — Event Handling
- [ ] `actor_ref_round_trip` — a `ractor::ActorRef` can send and receive a message.

### Layer 4 — Provider Replay / Mock-Tool E2E
- [ ] N/A.

## Files touched

- `crates/runie-core/src/actors/session/messages.rs`
- `crates/runie-core/src/actors/config/messages.rs`
- `crates/runie-core/src/actors/permission/messages.rs`
- `crates/runie-core/src/actors/provider/messages.rs`
- `crates/runie-core/src/actors/ractor_adapter.rs`
- `crates/runie-core/src/actors/mod.rs`

## Notes

- `GenericActorHandle` may be used only by dead wrappers; delete it if unused.
