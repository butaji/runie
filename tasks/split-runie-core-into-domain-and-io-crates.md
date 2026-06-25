# Split runie-core into domain and io crates

**Status**: todo
**Milestone**: R5
**Category**: Architecture / Refactoring
**Priority**: P2

**Depends on**: finish-io-migration, delete-async-io-bridge, fold-state-into-model-state, rename-core-ui-to-view
**Blocks**: gate-or-move-single-consumer-core-modules, consolidate-config-modules-into-dir, unify-duplicate-module-names-core-tui

## Description

Split `runie-core` into two crates:
- `runie-domain` — pure domain logic, no IO
- `runie-io` — async IO actors and file/network operations

This enables headless mode and better separation of concerns.

## Acceptance Criteria

- [ ] `runie-domain` crate with pure domain logic
- [ ] `runie-io` crate with IO actors
- [ ] `runie-core` re-exports both
- [ ] Headless mode works with `runie-domain` only
- [ ] `cargo test --workspace` passes

## Tests

### Layer 1 — State/Logic
- [ ] `domain_crate_has_no_tokio_imports`

### Layer 2 — Event Handling
- [ ] N/A

### Layer 3 — Rendering
- [ ] N/A

### Layer 4 — Provider Replay / Mock-Tool E2E
- [ ] `headless_mode_works_without_io_crate`

## Files touched

- `crates/runie-domain/` (new)
- `crates/runie-io/` (new)
- `crates/runie-core/Cargo.toml`

## Notes

- Large refactoring task
- Requires `finish-io-migration` first
- See architecture docs for crate map
