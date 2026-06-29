# Delete dead actor handle wrappers

**Status**: done
**Milestone**: R6
**Category**: Architecture / Actors
**Priority**: P1

**Depends on**: migrate-production-actors-to-ractor
**Blocks**: actually-collapse-actor-handles-to-typed-map

## Description

The legacy handle wrappers listed in the task description (`SessionActorHandle`, `PersistenceActorHandle`, `SessionStoreActorHandle`, `ConfigActorHandle`, `PermissionActorHandle`, `ProviderActorHandle::legacy_tx`, `GenericActorHandle`) do not exist in the codebase — they were already deleted during the migration to `ractor`.

The remaining handle types are all active:
- `RactorHandle<Msg>` — generic handle used throughout (`ractor_adapter.rs`)
- `RactorSessionHandle` — session actor handle (`session/ractor_session_handle.rs`)
- `RactorPermissionHandle` — permission actor handle (`permission/ractor_permission.rs`)
- `RactorTurnHandle` — turn actor handle (`turn/ractor_turn.rs`)

## Acceptance Criteria

- [x] Identify all dead wrappers (no production callers). — Confirmed: wrappers named in task do not exist.
- [x] Delete them and their trait implementations. — N/A.
- [x] Update any tests that used them to use `ractor::ActorRef`. — N/A.
- [x] `cargo test --workspace` succeeds after the change. — Already verified.
- [x] `cargo check --workspace` succeeds with no new warnings. — Already verified.

## Tests

### Layer 1 — State/Logic
- [x] N/A — no dead wrappers remain.

### Layer 2 — Event Handling
- [x] N/A.

### Layer 4 — Provider Replay / Mock-Tool E2E
- [x] N/A.

## Files touched

- None — the wrappers were already removed.

## Notes

- `GenericActorHandle` is referenced nowhere in the codebase (grep confirms).
- `RactorHandle<Msg>` is actively used in `ActorHandles` and all actor spawn paths.
- `ProviderActorHandle::legacy_tx` is not present — `RactorProviderHandle` is the current type.
