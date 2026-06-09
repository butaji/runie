# Configurable keybindings

**Status**: done
**Milestone**: R1
**Category**: Configuration

## Description

Load keybindings from `keybindings.json` (or TOML config section). Map keys to
`CoreEvent` variants without actor infrastructure.

## Acceptance Criteria

- [x] Parse `~/.runie/keybindings.json` at startup
- [x] Default keybindings (hardcoded fallback)
- [x] Custom key → event mapping via JSON
- [x] Reload on config change (via ConfigAgent integration)

## Implementation

### Files
- `crates/runie-core/src/keybindings.rs` — Keybindings module

### Key Functions
- `default_keybindings()` — Returns hardcoded fallback keybindings
- `load_keybindings(path)` — Load from file or fall back to defaults
- `parse_keybindings_json(content)` — Parse JSON config with overrides
- `validate_key_combo(combo)` — Validate key combo format

### Keybindings Format (JSON)
```json
{
    "ctrl+x": "Quit",
    "ctrl+z": "Undo",
    "enter": "Submit"
}
```

Values are event names from `Event` enum. Supports:
- Control: `ctrl+c`, `ctrl+j`, etc.
- Alt: `alt+enter`, `alt+b`, etc.
- Shift: `shift+enter`
- Plain: `enter`, `escape`, `up`, `down`, etc.

## Tests

### Layer 1 — State/Logic
- [x] `default_keybindings_has_common_keys` — Verify defaults include common keys
- [x] `load_keybindings_falls_back_to_defaults` — Fallback when file missing
- [x] `parse_keybindings_json_with_overrides` — JSON overrides work
- [x] `parse_keybindings_json_invalid_json` — Error handling
- [x] `validate_key_combo_valid` — Valid combos accepted
- [x] `validate_key_combo_invalid` — Invalid combos rejected
- [x] `parse_key_combo_*` — Key combo parsing

All 11 keybindings tests pass.
