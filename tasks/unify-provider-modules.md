# Unify Provider Modules

**Status**: done
**Milestone**: R4
**Category**: Configuration
**Priority**: P1

**Depends on**: event-taxonomy-for-actor-state-sync
**Blocks**: move-provider-catalog-to-provider-crate, consolidate-settings-providers-dialog

## Description

Consolidate scattered provider-related code into a unified `provider/` module. Currently provider logic is split between `login_config/`, `update/dialog/provider_*`, `actors/provider/`, etc.

## Implementation Summary

### Completed Work (2026-06-25)

- ✅ All provider logic consolidated into `crates/runie-core/src/provider/`:
  - `mod.rs` - module exports
  - `dialog.rs` - provider dialog (providers dialog, model editor)
  - `registry.rs` - provider registry
  - `registry_data.rs` - provider metadata
  - `provider_trait.rs` - Provider trait
  - `config.rs` - provider configuration
- ✅ `login_config/` is now a thin re-export module that forwards to `crate::provider`
- ✅ Provider toggle logic consolidated into `update/dialog/toggles.rs`
- ✅ Settings → Providers navigation works via `dialog/toggles.rs`

### Remaining Items

- `move-provider-catalog-to-provider-crate` will move catalog to runie-provider crate

## Acceptance Criteria

- [x] All provider logic in `crates/runie-core/src/provider/`
- [x] Provider dialog consolidated
- [x] Settings → Providers navigation works
- [x] `cargo test --workspace` passes

## Tests

### Layer 1 — State/Logic
- [x] `provider_catalog_lists_all_providers` (existing tests verify)

### Layer 2 — Event Handling
- [x] `provider_selection_intent_works` (toggles.rs tests verify)

### Layer 3 — Rendering
- [x] `providers_dialog_renders_all_providers` (existing tests verify)

### Layer 4 — Provider Replay / Mock-Tool E2E
- [x] N/A

## Files touched

- `crates/runie-core/src/provider/` (consolidated)
- `crates/runie-core/src/login_config/` (thin re-export)
- `crates/runie-core/src/update/dialog/toggles.rs` (consolidated toggle logic)

## Notes

- Main goal is consolidation, not new features
- Follow the existing provider trait in `runie-provider`
- The catalog will be moved to `runie-provider` in a separate task
