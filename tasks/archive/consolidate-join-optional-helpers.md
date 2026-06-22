# Consolidate join_optional Helpers

**Status**: done
**Milestone**: R3
**Category**: Core / State
**Priority**: P2

**Depends on**: none
**Blocks**: none

## Description

Two similar helpers in `agents.rs`:
```rust
fn join_optional(list: &Option<Vec<String>>) -> String { ... }  // comma-space
fn join_optional_csv(list: &Option<Vec<String>>) -> String { ... }  // comma only
```

Combine into one: `fn join_optional(list: &Option<Vec<String>>, sep: &str) -> String`

## Acceptance Criteria

- [x] Add parameterized `join_optional` to utils
- [x] Replace both call sites
- [x] `cargo test --workspace` succeeds

## Tests

### Layer 1 — State/Logic
- [x] `test_join_optional_default_sep` — verifies ", " separator
- [x] `test_join_optional_custom_sep` — verifies custom separator
- [x] `test_join_optional_empty` — verifies None returns empty string

### Layer 2 — Event Handling
- [x] N/A

### Layer 3 — Rendering
- [x] N/A

### Layer 4 — Smoke / Crash
- [x] N/A

## Files touched

- `crates/runie-core/src/commands/dsl/handlers/agents.rs`

## Notes

Trivial fix — 2 lines saved but improves consistency.
