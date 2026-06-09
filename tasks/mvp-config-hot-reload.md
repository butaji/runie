# Hot reload on config change

**Status**: done
**Milestone**: R1
**Category**: Configuration

## Description

Reload configuration when files change. Moved from MVP to R1 because TOML
config parsing is done; hot reload is infrastructure, not MVP-critical.

## Acceptance Criteria

- [x] File watcher (polling-based via ConfigFileState)
- [x] ConfigChanged events emitted to bus
- [x] Re-parse config on change
- [x] Actors apply changes without restart (via ConfigChanged events)

## Implementation

### Files
- `crates/runie-core/src/actors/config_agent.rs` — ConfigAgent actor
- `crates/runie-core/src/event_bus.rs` — ConfigChanged event and ConfigValue types

### Architecture

1. **ConfigFileState** tracks file path, mtime, and content hash
2. **check_and_update()** polls file for changes (mtime + content hash)
3. **ConfigChanged** event emitted with path and parsed HashMap<String, ConfigValue>
4. **run_config_agent()** runs actor loop with configurable poll interval

### ConfigValue Types
- String, Integer, Float, Boolean
- Array (nested arrays)
- Object (nested tables)

## Tests

### Layer 1 — State/Logic
- [x] `parse_config_file` — parse TOML and extract values
- [x] `config_file_state_detects_change` — mtime + hash change detection
- [x] Helper functions: `get_string`, `get_bool`, `get_integer`, `get_nested`

### Layer 2 — Event Handling
- [x] `test_spawn_config_agent` — actor spawning

All 10 config tests pass.
