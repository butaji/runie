# Consolidate settings + providers dialog modules

**Status**: todo
**Milestone**: R4
**Category**: Configuration
**Priority**: P1

**Depends on**: unify-provider-modules
**Blocks**: none

## Description

Settings and providers management UI logic is split across three modules with overlapping concerns:

- `settings.rs` (77 LOC) — settings dialog data model.
- `providers_dialog.rs` (96 LOC) — providers management dialog builder.
- `update/settings_dialog.rs` (296 LOC) — settings dialog event handlers.

Plus `update/dialog/provider_model_toggle.rs` (118) handles provider model toggling. None of these are in a shared directory. The result: "settings" lives in 2 places (`settings.rs` + `update/settings_dialog.rs`), "providers dialog" lives in 2 places (`providers_dialog.rs` + `update/dialog/provider_model_toggle.rs`), and reading the full settings/providers flow requires jumping across 4 files in 3 directories.

Consolidate into a single `settings/` directory: `settings/mod.rs`, `settings/dialog.rs` (data model + builder), `settings/handlers.rs` (event handlers). Folds `providers_dialog.rs` and `update/dialog/provider_model_toggle.rs` in.

## Acceptance Criteria

- [ ] `settings.rs`, `providers_dialog.rs`, `update/settings_dialog.rs` deleted from src root / `update/`.
- [ ] New `settings/` dir: `mod.rs`, `dialog.rs` (data + builder), `handlers.rs` (event handlers).
- [ ] `update/dialog/provider_model_toggle.rs` folded into `settings/handlers.rs`.
- [ ] `lib.rs` exports `pub mod settings;` and re-exports the same public API.
- [ ] `update/mod.rs` no longer declares `mod settings_dialog;`.
- [ ] `arch_guardrails.rs` path strings updated if affected.
- [ ] `cargo test --workspace` succeeds.
- [ ] `cargo check --workspace` succeeds with no new warnings.

## Tests

### Layer 1 — State/Logic
- [ ] `settings_dialog_data_model_unchanged` — settings dialog struct fields identical after move.
- [ ] `providers_dialog_builder_unchanged` — providers dialog panel builder produces the same `Panel` after move.

### Layer 2 — Event Handling
- [ ] `settings_toggle_opens_dialog` — `ToggleSettingsDialog` opens the settings panel.
- [ ] `provider_model_toggle_persists` — toggling a model in the providers dialog emits `ConfigMsg::SetProviderModels`.

### Layer 3 — Rendering
- [ ] `settings_dialog_renders_after_consolidation` — existing TUI settings render test passes.

### Layer 4 — Smoke / Crash
- [ ] `cargo test --workspace` green confirms all import paths resolved.

## Files touched

- `crates/runie-core/src/settings.rs` → delete (move to `settings/mod.rs` + `settings/dialog.rs`)
- `crates/runie-core/src/providers_dialog.rs` → delete (fold into `settings/dialog.rs`)
- `crates/runie-core/src/update/settings_dialog.rs` → delete (move to `settings/handlers.rs`)
- `crates/runie-core/src/update/dialog/provider_model_toggle.rs` → delete (fold into `settings/handlers.rs`)
- `crates/runie-core/src/settings/` → new (`mod.rs`, `dialog.rs`, `handlers.rs`)
- `crates/runie-core/src/lib.rs` — update module declarations
- `crates/runie-core/src/update/mod.rs` — remove `mod settings_dialog;`
- `crates/runie-core/tests/arch_guardrails.rs` — update paths

## Notes

Depends on `unify-provider-modules` (which moves `providers_dialog.rs` into `provider/dialog.rs`). If that task lands first, this task absorbs `provider/dialog.rs` into `settings/dialog.rs` instead of the root `providers_dialog.rs`. The net result is the same: one `settings/` dir. Rejected alternative: keep settings and providers as separate dirs — rejected because the providers dialog is accessed from the settings dialog and they share the `ConfigMsg` write path. This is the uncovered portion of finding #5; the login-flow portion is handled by `consolidate-login-flow-handlers` and the config-reload portion by `delete-config-reload-shim`.
