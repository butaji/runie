# Prune dead provider code and the `rig-core` dependency

**Status**: done
**Milestone**: R4
**Category**: Provider
**Priority**: P1
**Depends on**: none
**Blocks**: none

## Description

`crates/runie-provider` contains several unwired subtrees and an unused external dependency that inflate compile times and pull in duplicate transitive crates. `src/catalog/` and `src/registry/` are not declared in `lib.rs` (they re-export from `runie-core` instead). `src/rig_adapter.rs` defines `RigOpenAiProvider`, which is unused in production. The workspace-level `rig-core = "0.39"` dependency duplicates the workspace's `reqwest` version and causes multiple versions of transitive crates. This task deletes the dead modules and the `rig-core` dependency.

## Acceptance Criteria

- [x] `crates/runie-provider/src/catalog/` is deleted.
- [x] `crates/runie-provider/src/registry/` is deleted.
- [x] `crates/runie-provider/src/rig_adapter.rs` is deleted.
- [x] `rig-core` is removed from workspace `Cargo.toml` `[dependencies]`/`[workspace.dependencies]`.
- [x] `rig-core` is removed from `crates/runie-provider/Cargo.toml`.
- [x] `crates/runie-provider/src/lib.rs` no longer exports or references the deleted modules or `RigOpenAiProvider`.
- [x] No production or test code references `RigOpenAiProvider`, `rig_adapter`, `catalog`, or `registry`.
- [x] `cargo test --workspace` succeeds after the change.
- [x] `cargo check --workspace` succeeds with no new warnings.

## Tests

### Layer 1 — State/Logic
- [x] `no_rig_adapter_references` — Greps the workspace and asserts no references to `RigOpenAiProvider`, `rig_adapter`, `catalog`, or `registry` remain.

### Layer 2 — Event Handling
- N/A — No event handling changes.

### Layer 3 — Rendering
- N/A — No rendering changes.

### Layer 4 — Provider Replay / Mock-Tool E2E
- [x] `provider_crate_builds_without_rig_core` — Builds the workspace and asserts `cargo tree` no longer contains `rig-core`.

## Files touched

- `Cargo.toml` (removed `rig-core` from workspace.dependencies)
- `crates/runie-provider/Cargo.toml` (removed `rig-core` dependency)
- `crates/runie-provider/src/lib.rs` (removed `pub mod rig_adapter;`)
- `crates/runie-provider/src/catalog/` (deleted)
- `crates/runie-provider/src/registry/` (deleted)
- `crates/runie-provider/src/rig_adapter.rs` (deleted)

## Notes

The `catalog/` and `registry/` modules were dead code because `lib.rs` already re-exports from `runie_core::model_catalog` and `runie_core::provider::registry`. The local copies were never wired into the public API.
