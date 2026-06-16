# Adopt `notify` for Config File Watching

**Status**: done
**Milestone**: R3
**Category**: Configuration
**Priority**: P1

**Depends on**: (none)
**Blocks**: (none)

## Description

Replace the 2-second polling loop in `crates/runie-core/src/config_reload/watcher.rs` with the `notify` crate. `notify` provides cross-platform filesystem events and integrates cleanly with a `tokio` task that emits into the existing config-reload event channel.

## Acceptance Criteria

- [ ] `notify` and `notify-debouncer-mini` (or `notify-debouncer-full`) are added as dependencies.
- [ ] `config_reload/watcher.rs` no longer polls; it uses `RecommendedWatcher` or `new_debouncer`.
- [ ] Config changes trigger reload near-instantly (debounced to avoid spurious reloads).
- [ ] Existing `Config::load_from` and change-classification logic is preserved.
- [ ] `cargo test --workspace` succeeds.

## Tests

### Layer 1 — State/Logic
- [ ] `watcher_emits_config_changed_on_file_write` — write to `config.toml`, watcher emits event.
- [ ] `watcher_debounces_rapid_writes` — multiple rapid writes produce one reload event.

### Layer 2 — Event Handling
- [ ] `config_changed_event_reaches_dispatcher` — emitted event is processed by the update dispatcher.

### Layer 3 — Rendering
N/A.

### Layer 4 — Smoke / Crash
- [ ] `smoke_config_hot_reload` — change theme in config, UI updates without restart.

## Files touched

- `crates/runie-core/Cargo.toml`
- `crates/runie-core/src/config_reload/watcher.rs`

## Notes

- Keep the watcher in a dedicated `tokio` task and send events through the existing channel.
- See `docs/CRATE_DECISIONS.md`.
