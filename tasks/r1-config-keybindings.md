# Configurable keybindings

**Status**: done
**Milestone**: R1
**Category**: Configuration

## Description

Load keybindings from `~/.runie/keybindings.json` (or TOML config section). Map keys to
`CoreEvent` variants.

## Current State

**Implemented (but not wired):**
- `crates/runie-core/src/keybindings.rs` (233 lines)
- Parses `~/.runie/keybindings.json`
- Returns `HashMap<String, String>` (key combo → event name)
- 11 unit tests pass

**Not implemented:**
- `main.rs` never calls `load_keybindings()` — all keys still hardcoded in `map_key_event()`
- Custom keybindings have **zero effect** on runtime

## Acceptance Criteria

- [x] Parse `~/.runie/keybindings.json` at startup
- [x] Default keybindings (hardcoded fallback)
- [x] Custom key → event mapping via JSON
- [x] **Wire into runtime** — replace `map_key_event()` with loaded bindings lookup

## Implementation

### Files
- `crates/runie-core/src/keybindings.rs` — Keybindings module
- `crates/runie-term/src/main.rs` — needs to call `load_keybindings()` and use it

### Key Functions
- `default_keybindings()` — Returns hardcoded fallback keybindings
- `load_keybindings(path)` — Load from file or fall back to defaults
- `parse_keybindings_json(content)` — Parse JSON config with overrides

### Keybindings Format (JSON)
```json
{
    "ctrl+x": "Quit",
    "ctrl+z": "Undo",
    "enter": "Submit"
}
```

## Tests

### Layer 1 — State/Logic
- [x] `default_keybindings_has_common_keys` — Verify defaults include common keys
- [x] `load_keybindings_falls_back_to_defaults` — Fallback when file missing
- [x] `parse_keybindings_json_with_overrides` — JSON overrides work
- [x] `parse_keybindings_json_invalid_json` — Error handling

### Layer 2 — Event Handling
- [x] `custom_keybinding_overrides_default` — loaded map overrides hardcoded
- [x] `unknown_keybinding_falls_back_to_default` — unmapped keys fall through
- [x] `key_event_to_combo_*` — combo string generation for all modifier combinations

## Notes

- Module is complete. Only wiring remains (~20 lines in main.rs).
- See `docs/SHIP_REVIEW_3.md`.
