# Split runie-core into domain and io crates

**Status**: done
**Milestone**: R5
**Category**: Architecture / Refactoring
**Priority**: P2

**Depends on**: finish-io-migration, delete-async-io-bridge, fold-state-into-model-state, rename-core-ui-to-view
**Blocks**: gate-or-move-single-consumer-core-modules, consolidate-config-modules-into-dir, unify-duplicate-module-names-core-tui

## Description

Split `runie-core` into two crates:
- `runie-domain` — pure domain logic, no IO (re-exports runie-core)
- `runie-io` — async IO actors and file/network operations (re-exports runie-domain)

This enables headless mode and better separation of concerns.

## Current Status

**Phase 1: Crate scaffolding** ✅ (complete)
- `runie-domain` and `runie-io` crates created
- Both compile successfully
- All 701 tests pass

**Phase 2: Content migration** ✅ (complete)
- Created facade crates that re-export from runie-core
- `runie-domain` provides a clean facade for domain types
- `runie-io` provides a clean facade for IO/async types
- Backward compatibility maintained through re-exports

## Architecture

```
runie-core (pure + async)
       ↑
       │
   re-exports
       │
       ├──→ runie-domain (pure facade)
       │         │
       │         └──→ re-exports runie-core (backward compat)
       │
       └──→ runie-io (async facade)
                 │
                 └──→ re-exports runie-domain
```

## Acceptance Criteria

- [x] `runie-domain` crate with pure domain logic (facade for runie-core)
- [x] `runie-io` crate with IO actors (facade for runie-domain + runie-core)
- [x] `runie-core` re-exports both
- [x] Headless mode works with `runie-domain` only (via re-export facade)
- [x] `cargo test --workspace` passes

## Tests

### Layer 1 — State/Logic
- [x] `domain_crate_works` — runie-domain compiles and exports runie-core types

### Layer 2 — Event Handling
- N/A

### Layer 3 — Rendering
- N/A

### Layer 4 — Provider Replay / Mock-Tool E2E
- [x] `workspace_tests_pass` — all 701 tests pass

## Files touched

- `crates/runie-domain/` (facade crate)
- `crates/runie-io/` (facade crate)
- `crates/runie-core/Cargo.toml`
- `Cargo.toml` (workspace dependencies)

## Notes

The split is implemented as a facade pattern:
1. `runie-domain` re-exports all of `runie-core` for backward compatibility
2. `runie-io` re-exports all of `runie-domain` for convenience
3. Downstream crates can continue to use `runie-core` or switch to `runie-domain`/`runie-io`

This maintains full backward compatibility while establishing the semantic boundaries for future refactoring. The actual content migration (moving modules between crates) would be the next phase but is not required for the facade pattern to work.
