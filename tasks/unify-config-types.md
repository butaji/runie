# Unify Config Types

**Status**: todo
**Milestone**: R3
**Category**: Configuration
**Priority**: P0

**Depends on**: (none)
**Blocks**: (none)

## Description

The same `~/.runie/config.toml` is parsed by three different structs:
`runie-provider::Config`, `runie-core::config_reload::Config`, and the
login-config helpers in `runie-core::login_config`. `ModelsSection` is
defined twice with different fields, `default_model()` is copy-pasted, and
`config_path()` exists in multiple places. Any format change must be edited
in multiple files and can drift silently.

This task creates a single canonical config type and makes all crates
consume it.

## Acceptance Criteria

- [ ] A single `Config` type owns the TOML schema for `~/.runie/config.toml`.
- [ ] `runie-provider` depends on the canonical type (or a new
  `runie-config` crate) instead of defining its own.
- [ ] `runie-core::config_reload` no longer duplicates `ModelsSection`,
  `PromptsSection`, `TelemetrySection`, `TruncationSection`, or `UiSection`.
- [ ] `login_config` uses the same canonical config type for provider
  sections.
- [ ] `config_path()` and `default_model()` have exactly one implementation.
- [ ] All existing config tests pass without duplication.
- [ ] `cargo test --workspace` succeeds.

## Tests

### Layer 1 — State/Logic
- [ ] `config_load_parses_all_sections` — canonical `Config::load_from`
  parses provider, model, theme, keybindings, truncation, and UI sections.
- [ ] `config_defaults_when_missing` — missing file returns defaults.
- [ ] `provider_and_core_see_same_default_model` — provider crate and core
  crate read the same default model from the same config value.

### Layer 2 — Event Handling
- [ ] `config_change_event_emits_switch_model` — hot-reload still emits
  `ModelConfigEvent::SwitchModel` when the canonical config changes.

## Files touched

- `crates/runie-provider/src/config.rs`
- `crates/runie-core/src/config_reload/types.rs`
- `crates/runie-core/src/login_config.rs`
- `crates/runie-core/src/config_migrate.rs`
- `crates/runie-core/Cargo.toml` (if new crate or dependency added)
- `crates/runie-provider/Cargo.toml`

## Notes

Supersedes `tasks/consolidate-config-reload-types.md`.

Keep the public API of each crate stable where possible by re-exporting the
canonical type under the existing module paths.
