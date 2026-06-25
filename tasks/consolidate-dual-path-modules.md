# Consolidate Dual-Path Modules

**Status**: todo
**Milestone**: R4
**Category": Architecture / Refactoring
**Priority**: P2

**Depends on**: event-taxonomy-for-actor-state-sync
**Blocks**: relocate-loose-tests-files

## Description

Consolidate modules that exist in both `src/` and `tests/` into a single canonical location. Currently some modules have duplicate definitions in test files for isolation.

## Acceptance Criteria

- [ ] No duplicate module definitions
- [ ] Tests import from canonical location
- [ ] `cargo test --workspace` passes

## Tests

### Layer 1 — State/Logic
- [ ] `no_duplicate_modules`

### Layer 2 — Event Handling
- [ ] N/A

### Layer 3 — Rendering
- [ ] N/A

### Layer 4 — Provider Replay / Mock-Tool E2E
- [ ] N/A

## Files touched

- Various `tests/` files
- Various `src/` modules

## Notes

- Pattern: `tests/foo.rs` should be `tests/foo/mod.rs` if module, or tests inline
- Avoid `mod foo` in test files that duplicate `src/foo.rs`
