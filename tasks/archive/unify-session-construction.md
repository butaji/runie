# Unify Session Construction

**Status**: todo
**Milestone**: R3
**Category**: Sessions
**Priority**: P2

**Depends on**: none
**Blocks**: none

## Description

Session save and export both construct a Session from AppState with nearly identical code (13 lines each) in `session/io.rs:22-34` and `session/io.rs:117-133`.

Extract to `Session::from_state(state: &AppState, name: String)`.

## Acceptance Criteria

- [ ] Add `Session::from_state(&AppState, String) -> Self`
- [ ] Replace both save and export call sites
- [ ] `cargo test --workspace` succeeds
- [ ] ~30 LOC reduced

## Tests

### Layer 1 — State/Logic
- [ ] Existing session save/export tests pass

### Layer 2 — Event Handling
- [ ] Save/export command tests pass

### Layer 3 — Rendering
- [ ] N/A

### Layer 4 — Smoke / Crash
- [ ] N/A

## Files touched

- `crates/runie-core/src/session/mod.rs` (add impl)
- `crates/runie-core/src/commands/dsl/handlers/session/io.rs`

## Notes

Pairs with `extract-session-restore-helper` — together they unify session serialization/deserialization.
