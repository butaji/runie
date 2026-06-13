# Consolidate Config Types and Remove Polling Sleeps

**Status**: todo
**Milestone**: R3
**Category**: Configuration
**Priority**: P1

## Description

`crates/runie-core/src/config_reload.rs` duplicates config structs from
`runie-provider` “to avoid circular dependency”:

- `ModelsSection`
- `PromptsSection`
- `TelemetrySection`
- `TruncationSection`
- `UiSection`

This duplication means a config format change must be edited in two
places. The watcher also polls `config.toml` every 2 seconds, and the
unit/integration tests rely on `tokio::time::sleep(4s)` and
`tokio::time::sleep(3s)`, violating the project rule against artificial
delays in automatic tests.

## Acceptance Criteria

- [ ] Shared config types are moved to `runie-core` (or a new
  `runie-config` crate) and `runie-provider`/`runie-core` consume them.
- [ ] `config_reload.rs` no longer duplicates structs already defined
  elsewhere.
- [ ] The file watcher uses a filesystem notify mechanism
  (`notify` crate or a deterministic test hook) instead of polling.
- [ ] All `tokio::time::sleep` calls are removed from config-reload
  tests; tests wait on channels/events deterministically.
- [ ] `cargo test --workspace` still passes and is faster than before.

## Tests

### Layer 1 — State/Logic
- [ ] `config_load_parses_toml` — `Config::load_from` parses all
  sections.
- [ ] `config_defaults_when_missing` — missing file returns defaults.

### Layer 2 — Event Handling
- [ ] `config_watcher_emits_event_on_change` — writing a new config
  produces a `SwitchModel` event without sleeping.
- [ ] `config_watcher_no_event_when_unchanged` — re-reading the same
  config emits no event.

### Layer 3 — Rendering
- [ ] No rendering changes.

### Layer 4 — Smoke
- [ ] `./dev.sh` still hot-reloads provider/model changes from
  `~/.runie/config.toml`.

## Notes

**Why not keep polling:**
Polling wastes CPU, adds latency, and forces tests to sleep. A notify-
based watcher is deterministic if tests trigger writes and await a
channel.

**Out of scope:**
- Changing the config file format.
- Adding new config sections.

## Verification

```bash
cargo test -p runie-core --lib config_reload
grep -R "tokio::time::sleep" crates/runie-core/src/config_reload*
# Expected: no matches

cargo test --workspace
```
