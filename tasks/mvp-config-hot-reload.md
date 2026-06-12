# Hot reload on config change

**Status**: done
**Milestone**: R1
**Category**: Configuration

## Description

Reload configuration when files change.

## Current State

**Implemented.** `crates/runie-core/src/config_reload.rs` (276 lines) provides a
polling-based file watcher. Wired into `main.rs` via
`config_reload::spawn_config_watcher()`. Polls `~/.runie/config.toml` every 2
seconds and emits `SwitchModel` events when provider or model changes.

## Acceptance Criteria

- [x] File watcher (polling-based, 2-second interval)
- [x] Re-parse config on change
- [x] Apply provider/model changes without restart (via `SwitchModel` event)

## Tests

- [x] Layer 1 — `config_load_parses_toml` — parse TOML config
- [x] Layer 1 — `config_load_defaults_when_missing` — fallback behavior
- [x] Layer 1 — `config_load_uses_default_model_from_models_section` — models.default precedence
- [x] Layer 1 — `config_path_returns_expected_path` — path construction
- [x] Layer 2 — `config_watcher_detects_initial_change` — emits SwitchModel on startup
- [x] Layer 2 — `config_watcher_parses_toml_changes` — detects provider/model changes
- [x] Layer 2 — `config_changed_applies_provider` — SwitchModel event updates state

## Notes

- Uses polling (2-second interval) instead of `notify` crate to avoid extra dependency.
- `Config` struct duplicates `runie_provider::Config` fields to avoid circular dependency.
- See `docs/SHIP_REVIEW_3.md`.
