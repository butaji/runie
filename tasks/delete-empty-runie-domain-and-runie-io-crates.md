# Delete empty `runie-domain` and `runie-io` facade crates

**Status**: todo
**Milestone**: R4
**Category**: Architecture / Refactoring
**Priority**: P1
**Depends on**: none
**Blocks**: none

## Description

`crates/runie-domain/src/lib.rs` is a 27-line facade that only does `pub use runie_core::*`, and `crates/runie-io/src/lib.rs` is a 21-line facade that only does `pub use runie_domain;`. Their `Cargo.toml` files list numerous dependencies that are never used, and no other crate in the workspace depends on either crate. This task removes both crates entirely, updates the workspace `members` list, and cleans any residual references.

## Acceptance Criteria

- [ ] The `crates/runie-domain/` directory is removed in its entirety.
- [ ] The `crates/runie-io/` directory is removed in its entirety.
- [ ] Workspace `Cargo.toml` no longer lists `runie-domain` or `runie-io` in `workspace.members` or `workspace.dependencies`.
- [ ] No crate manifest, `use` statement, README, or build script references `runie-domain` or `runie-io`.
- [ ] `cargo test --workspace` succeeds after the change.
- [ ] `cargo check --workspace` succeeds with no new warnings.

## Tests

### Layer 1 — State/Logic
- N/A — Deleting facade crates has no internal state/logic to test.

### Layer 2 — Event Handling
- N/A — No event handling changes.

### Layer 3 — Rendering
- N/A — No rendering changes.

### Layer 4 — Provider Replay / Mock-Tool E2E
- [ ] `workspace_compiles_without_domain_io_crates` — Builds the workspace and asserts `runie-domain`/`runie-io` crates are gone and all tests pass.

## Files touched

- `Cargo.toml` (removed `runie-domain` and `runie-io` from members and dependencies)
- `crates/runie-domain/` (deleted)
- `crates/runie-io/` (deleted)

## Notes

These crates were intended as abstraction boundaries but became empty pass-throughs. Removing them reduces dependency duplication and workspace surface area.
