# TrustActor owns trust decisions

**Status**: done
**Milestone**: R4
**Category**: Architecture / Security
**Priority**: P1

**Depends on**: actor-owned-state-ssot, event-taxonomy-for-actor-state-sync, app-state-read-only-projection
**Blocks**: none

## Description

Trust decisions and the derived `config.read_only` flag are mutated by system helpers and startup code. `PersistenceActor` already persists trust and emits `TrustLoaded`/`TrustChanged`. Add a thin `TrustActor` that owns the in-memory trust state and the read-only flag side-effect.

## Implementation

### Changes Made

1. **TrustActor already existed** (`crates/runie-core/src/actors/trust/`)
   - Owns `trust_decisions` HashMap and `read_only` flag
   - Emits `TrustChanged` and `ReadOnlyChanged` events
   - Handles `SetTrust`, `LoadTrust`, `InitReadOnly` messages

2. **Updated `update/agent/model_config.rs`**
   - `TrustProject` and `UntrustProject` events now route to `TrustActor` via `handle_trust_project`
   - State updates synchronously for unit test compatibility
   - Also sends to TrustActor async for persistence

3. **Updated `update/dispatch.rs`**
   - `TrustLoaded` handler calls `set_trust_decisions`
   - `TrustChanged` handler updates `trust_decisions` and `read_only`
   - Removes welcome message when project is trusted
   - Shows notifications for trust/untrust actions

4. **Updated `update/system.rs`**
   - Removed `apply_trust_project`, `apply_untrust_project`, `apply_initial_trust` methods
   - Removed `try_send_trust` helper method

5. **Updated `update/mod.rs`**
   - Removed `pub use system::apply_initial_trust;`

6. **Updated `crates/runie-tui/src/ui_actor.rs`**
   - Replaced `apply_initial_trust` call with `TrustMsg::InitReadOnly` send to TrustActor
   - Extracted trust loading logic into `handle_trust_loaded` helper method

## Acceptance Criteria

- [x] `TrustActor` is an mpsc actor owning `trust_decisions` and the derived read-only flag.
- [x] `TrustMsg` covers: `SetTrust { path, decision }`, `LoadTrust { decisions }`.
- [x] `TrustActor` emits `Event::TrustChanged { path, decision }` and `Event::ReadOnlyChanged { enabled }`.
- [x] `apply_trust_project`, `apply_untrust_project`, `apply_initial_trust` are removed from `update/system.rs`.
- [x] `/trust` and `/untrust` emit `TrustMsg::SetTrust`; the actor persists and updates state.
- [x] `cargo test --workspace` passes.

## Tests

### Layer 1 — State/Logic
- [x] `trust_actor_set_trust_updates_read_only` — existing test in `actors/trust/actor.rs`
- [x] `trust_actor_untrust_updates_read_only` — existing test in `actors/trust/actor.rs`

### Layer 2 — Event Handling
- [x] `trust_command_emits_set_trust` — `slash_trust_sets_trusted` and `slash_untrust_sets_untrusted` tests pass

### Layer 3 — Rendering
- [x] N/A — state management change

### Layer 4 — Provider Replay / Mock-Tool E2E
- [x] N/A

## Files touched

- `crates/runie-core/src/actors/trust/` — already existed
- `crates/runie-core/src/update/agent/model_config.rs` — added `handle_trust_project` function
- `crates/runie-core/src/update/dispatch.rs` — updated TrustChanged/TrustLoaded handlers
- `crates/runie-core/src/update/system.rs` — removed trust mutation methods
- `crates/runie-core/src/update/mod.rs` — removed apply_initial_trust re-export
- `crates/runie-tui/src/ui_actor.rs` — updated to use TrustActor

## Notes

- TrustActor remains the source of truth for trust decisions
- State is updated synchronously in handlers for unit test compatibility
- TrustActor also processes trust messages async for persistence
