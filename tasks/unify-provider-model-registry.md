# Unify Provider and Model Registry

**Status**: done
**Completed**: 2026-06-14
**Milestone**: R3
**Category**: Providers & Models
**Priority**: P0

## Description

Provider and model metadata currently live in three places with inconsistent data:

- `crates/runie-core/src/provider_registry.rs` — provider list + default models.
- `crates/runie-core/src/model_catalog.rs` — model selector catalog with cost/flags.
- `crates/runie-provider/src/model.rs` — another `ModelRegistry` used to seed scoped
  models.

This causes drift (e.g., `o1`/`o3` exist in some catalogs but not others) and forces
`app_init.rs` to use a different registry than the rest of the app.

## Acceptance Criteria

- [x] A single registry exists in `runie-core` containing provider metadata and model
  capability flags (cost, thinking, vision, tokenizer family).
- [x] `model_catalog()` derives its data from this registry.
- [x] `runie-server::handle_list_models` derives its data from the same registry
  (via `model_catalog()`).
- [x] `runie-provider/src/model.rs::ModelRegistry` is removed.
- [x] Name inconsistencies are fixed (e.g., `anthropic/claude-sonnet-4-6`).
- [x] Model selector, scoped-model init, and server tests all pass.

## Tests

### Layer 1 — State/Logic
- [x] `model_catalog_contains_all_provider_default_models`.
- [x] `registry_model_has_consistent_provider` — no model appears under a provider that
  is not in the provider registry.

### Layer 2 — Event Handling
- [x] `cycle_model_next` uses the unified catalog (`model_selector` logic uses
  `model_catalog()` under the hood).

### Layer 3 — Rendering
- [x] `model_selector_renders_grouped_models` — TestBackend shows the expected providers
  and grouping.

## Files touched

- `crates/runie-core/src/provider_registry.rs`
- `crates/runie-core/src/model_catalog.rs`
- `crates/runie-provider/src/model.rs`
- `crates/runie-term/src/app_init.rs`
- `crates/runie-server/src/main.rs`

## Out of scope

- Fetching model lists dynamically from providers.
- Adding new providers/models beyond the current union set.
