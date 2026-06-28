# Consolidate settings + providers dialog modules

**Status**: done
**Milestone**: R4
**Category**: Configuration
**Priority**: P1

**Depends on**: unify-provider-modules, actor-owned-state-ssot, config-ssot-via-configactor
**Blocks**: none

## Description

Settings and providers management UI logic was split across three modules with overlapping concerns:

- `settings.rs` (77 LOC) — settings dialog data model.
- `providers_dialog.rs` — providers management dialog builder.
- `update/settings_dialog.rs` (328 LOC) — settings dialog event handlers.

## Implementation Summary

### Completed Work (2026-06-25)

- ✅ Moved `settings.rs` → `settings/mod.rs`
- ✅ Moved settings dialog logic → `settings/dialog.rs`
- ✅ Created `settings/mod.rs` that re-exports from `settings/dialog.rs`
- ✅ Created backward-compatible re-export at `update/settings_dialog.rs`
- ✅ Provider dialog already exists at `provider/dialog.rs`

### File Structure After Consolidation

```
crates/runie-core/src/settings/
├── mod.rs      (was settings.rs - data model + re-exports)
└── dialog.rs   (dialog builder logic)
```

## Acceptance Criteria

- [x] `settings.rs` → `settings/mod.rs`
- [x] Settings dialog logic → `settings/dialog.rs`
- [x] New `settings/` dir with `mod.rs` and `dialog.rs`
- [x] `update/settings_dialog.rs` kept as backward-compatible re-export
- [x] `cargo test --workspace` succeeds
- [x] `cargo check --workspace` succeeds with no new warnings

## Tests

### Layer 1 — State/Logic
- [x] `settings_dialog_data_model_unchanged` — settings dialog struct fields identical after move

### Layer 2 — Event Handling
- [x] `settings_toggle_opens_dialog` — existing tests verify
- [x] `provider_model_toggle_persists` — existing tests verify

### Layer 3 — Rendering
- [x] `settings_dialog_renders_after_consolidation` — existing tests verify

### Layer 4 — Smoke / Crash
- [x] `cargo test --workspace` green confirms all import paths resolved

## Files touched

- `crates/runie-core/src/settings.rs` → `crates/runie-core/src/settings/mod.rs`
- `crates/runie-core/src/update/settings_dialog.rs` → re-export (kept for backward compat)
- `crates/runie-core/src/settings/dialog.rs` → new (settings dialog logic)
- `crates/runie-core/src/settings/mod.rs` → updated (re-exports)

## Notes

- Provider dialog remains at `provider/dialog.rs` (already consolidated)
- Backward compatibility maintained via `update/settings_dialog.rs` re-export
