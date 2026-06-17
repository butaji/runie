# Unify Config Loading Entry Points

**Status**: todo
**Milestone**: R3
**Category**: Configuration
**Priority**: P2

**Depends on**: none
**Blocks**: none

## Description

Config loading uses 3 different patterns:
- `Config::load()` — `app_init.rs`
- `Config::load_from(path)` — `watcher.rs`, `system.rs`

Unify to single entry point with optional path override.

## Acceptance Criteria

- [ ] Determine canonical loading method
- [ ] Remove redundant variants
- [ ] Update all call sites
- [ ] `cargo test --workspace` succeeds

## Tests

### Layer 1 — State/Logic
- [ ] Config load tests pass

### Layer 2 — Event Handling
- [ ] Config change/reload tests pass

### Layer 3 — Rendering
- [ ] N/A

### Layer 4 — Smoke / Crash
- [ ] `scripts/smoke-tmux.sh` passes

## Files touched

- `crates/runie-core/src/app_init.rs`
- `crates/runie-core/src/config_reload/watcher.rs`
- `crates/runie-core/src/config.rs`
- `crates/runie-core/src/commands/dsl/handlers/system.rs`

## Notes

Low priority but reduces confusion about which method to use.
