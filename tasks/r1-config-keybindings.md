# Configurable keybindings

**Status**: todo
**Milestone**: R1
**Category**: Configuration

## Description

Load keybindings from `keybindings.json` (or TOML config section). Map keys to
`CoreEvent` variants without actor infrastructure.

## Acceptance Criteria

- [ ] Parse `~/.runie/keybindings.json` at startup
- [ ] Default keybindings (hardcoded fallback)
- [ ] Custom key → `CoreEvent` mapping
- [ ] Reload on config change (optional — can restart)

## Tests

- [ ] Layer 1 — `keybindings_load_default` — fallback when file missing
- [ ] Layer 1 — `keybindings_custom_map` — custom key produces correct event
- [ ] Layer 2 — `ctrl_custom_key_emits_event` — crossterm integration

## Notes

- No actor infrastructure needed. Read file at startup, store HashMap in AppState.
- See `docs/adr/0013-configurable-keybindings.md` for design.
