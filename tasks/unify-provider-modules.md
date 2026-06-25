# Unify Provider Modules

**Status**: todo
**Milestone**: R4
**Category**: Configuration
**Priority**: P1

**Depends on**: event-taxonomy-for-actor-state-sync
**Blocks**: move-provider-catalog-to-provider-crate, consolidate-settings-providers-dialog

## Description

Consolidate scattered provider-related code into a unified `provider/` module. Currently provider logic is split between `login_config/`, `update/dialog/provider_*`, `actors/provider/`, etc.

## Acceptance Criteria

- [ ] All provider logic in `crates/runie-core/src/provider/`
- [ ] Provider dialog consolidated
- [ ] Settings → Providers navigation works
- [ ] `cargo test --workspace` passes

## Tests

### Layer 1 — State/Logic
- [ ] `provider_catalog_lists_all_providers`

### Layer 2 — Event Handling
- [ ] `provider_selection_intent_works`

### Layer 3 — Rendering
- [ ] `providers_dialog_renders_all_providers`

### Layer 4 — Provider Replay / Mock-Tool E2E
- [ ] N/A

## Files touched

- `crates/runie-core/src/provider/` (new/modified)
- `crates/runie-core/src/login_config/` (moved/consolidated)
- `crates/runie-core/src/update/dialog/provider_*` (moved/consolidated)

## Notes

- Main goal is consolidation, not new features
- Follow the existing provider trait in `runie-provider`
