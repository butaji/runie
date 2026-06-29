# Normalize remaining `std::sync::Mutex`/`RwLock` to `parking_lot`

**Status**: todo
**Milestone**: R6
**Category**: Reliability
**Priority**: P1

**Depends on**: migrate-production-actors-to-ractor
**Blocks**: none

## Description

`harden-actors-against-mutex-poisoning.md` covered actor modules, but several actor-adjacent modules still use `std::sync::Mutex`/`RwLock` with explicit poison recovery: permissions, fff_indexer, and `runie-agent`. Replace them with `parking_lot::{Mutex,RwLock}` and delete poison-recovery code.

## Acceptance Criteria

- [ ] Replace `std::sync::Mutex` in `permissions/approval_registry.rs` and `permissions/sink.rs`.
- [ ] Replace `std::sync::RwLock` in `actors/fff_indexer/mod.rs`.
- [ ] Replace `std::sync::Mutex` in `runie-agent/src/actor.rs` and `runie-agent/src/subagent.rs`.
- [ ] Remove explicit poison recovery (`into_inner()` on poisoned lock etc.).
- [ ] `cargo test --workspace` succeeds after the change.
- [ ] `cargo check --workspace` succeeds with no new warnings.

## Tests

### Layer 1 — State/Logic
- [ ] `approval_registry_locks_with_parking_lot` — registry operations succeed.
- [ ] `subagent_state_no_poison_recovery` — subagent result collection works without poison handling.

### Layer 2 — Event Handling
- [ ] N/A.

### Layer 3 — Rendering
- [ ] N/A.

### Layer 4 — Provider Replay / Mock-Tool E2E
- [ ] `provider_replay_after_mutex_normalization` — turn completes after the change.

## Files touched

- `crates/runie-core/src/permissions/approval_registry.rs`
- `crates/runie-core/src/permissions/sink.rs`
- `crates/runie-core/src/actors/fff_indexer/mod.rs`
- `crates/runie-agent/src/actor.rs`
- `crates/runie-agent/src/subagent.rs`

## Notes

- `parking_lot` is already a workspace dependency.
- This is mechanical but touches sensitive concurrency code; add tests.
