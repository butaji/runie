# Hot reload on config change

**Status**: todo
**Milestone**: R1
**Category**: Configuration

## Description

Reload configuration when files change. Moved from MVP to R1 because TOML config parsing is done; hot reload is infrastructure, not MVP-critical.

## Acceptance Criteria

- [ ] File watcher (notify crate or polling)
- [ ] ConfigChanged events emitted to bus
- [ ] Actors apply changes without restart

## Tests

- [ ] Layer 1 — `config_reload_parses_new_values` — parse updated TOML
- [ ] Layer 2 — `config_changed_event_emitted_on_file_change` — bus integration
- [ ] Layer 4 — Smoke: modify config, verify behavior changes without restart
