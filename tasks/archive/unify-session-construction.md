# Unify Session Construction

**Status**: done
**Milestone**: R3
**Category**: Sessions
**Priority**: P2

**Depends on**: none
**Blocks**: none

## Description

Session save and export both construct a Session from AppState with nearly identical code (13 lines each) in `session/io.rs:22-34` and `session/io.rs:117-133`.

Extract to `Session::from_state(state: &AppState, name: String)`.

## Acceptance Criteria

- [x] Add `Session::from_state(&AppState, String) -> Self`
- [x] Replace both save and export call sites
- [x] `cargo test --workspace` succeeds
- [x] ~30 LOC reduced

## Tests

### Layer 1 — State/Logic
- [x] Existing session save/export tests pass

### Layer 2 — Event Handling
- [x] Save/export command tests pass

### Layer 3 — Rendering
- [ ] N/A

### Layer 4 — Smoke / Crash
- [ ] N/A

## Files touched

- `crates/runie-core/src/session.rs` (impl already existed)
- `crates/runie-core/src/update/command.rs` (uses Session::from_state)

## Notes

Pairs with `extract-session-restore-helper` — together they unify session serialization/deserialization.
