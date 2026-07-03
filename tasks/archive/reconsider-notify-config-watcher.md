# Reconsider notify config watcher

**Status**: done
**Milestone**: R4
**Category**: Configuration
**Priority**: P2

**Depends on**: none
**Blocks**: none

## Description

`notify` + `notify-debouncer-mini` were adopted in the done task `adopt-notify-config-watcher` to replace a 2s polling loop with OS file-watch events for `ConfigActor` hot-reload. Reversal argument under the YAGNI / stdlib / OS-features posture:

- Only one consumer: `crates/runie-core/src/actors/config/actor.rs`.
- `notify` pulls in platform-specific crates (`inotify` on Linux, `kqueue` on BSD, `FSEvents` on macOS) for a single config file.
- A 1s `tokio::time::interval` + `spawn_blocking(|| fs::metadata)` stat-poll is ~15 LOC of stdlib + tokio, debounces naturally by mtime comparison, and has no platform-specific failure modes (inotify watch descriptor exhaustion, FSEvents coalescing quirks).
- Config hot-reload latency budget is human-scale (seconds); 1s poll is indistinguishable from event-driven for the user.

Either (a) revert to stat-poll, or (b) document a concrete reason the event-driven approach is required (e.g. sub-second latency need, watch-descriptor budget not a concern) and keep.

## Acceptance Criteria

- [ ] Decision made: EITHER
  - (a) **Revert** — `ConfigActor` debounces config changes via a 1s `tokio::time::interval` + `spawn_blocking(|| fs::metadata(path))` mtime comparison; `notify` and `notify-debouncer-mini` removed from `runie-core/Cargo.toml` and `[workspace.dependencies]`; OR
  - (b) **Keep + document** — a concrete reason is written into `actors/config/actor.rs` module docs.
- [ ] If (a): `rg "notify::|notify_debouncer|DebouncedEvent" crates/` returns zero hits.
- [ ] If (a): `Cargo.lock` no longer pulls `notify`, `notify-debouncer-mini`, `inotify`, `kqueue`, `fsevent-sys` (or platform equivalents).
- [ ] Config hot-reload still fires within ~1.5s of an external edit to `~/.runie/config.toml`.
- [ ] `cargo check --workspace` succeeds with no new warnings.
- [ ] `cargo test --workspace` succeeds.

## Tests

### Layer 1 — State/Logic
- [ ] `mtime_changed_detects_edit` — given two `fs::metadata` snapshots, the helper correctly reports the second as newer.
- [ ] `debounce_drops_rapid_bursts` — three edits within 500ms produce exactly one reload, not three.

### Layer 2 — Event Handling
- [ ] `config_actor_reloads_on_external_edit` — after a stat-poll detects mtime change, `ConfigLoaded` is published with the new config.

### Layer 3 — Rendering
- N/A.

### Layer 4 — Smoke / Crash
- [ ] `smoke_config_hot_reload_under_1_5s` — write a new config to a temp path, assert `ConfigLoaded` arrives within 1500ms.
- [ ] `smoke_missing_config_path_does_not_panic` — pointing the actor at a non-existent path does not crash the poll loop.

## Files touched

- `crates/runie-core/src/actors/config/actor.rs` (rewrite watcher if option a)
- `crates/runie-core/Cargo.toml` (remove `notify`, `notify-debouncer-mini` if option a)
- `Cargo.toml` (remove workspace deps if option a)

## Notes

`adopt-notify-config-watcher` notes say "replaced 2s polling, debounced to 300ms". A revert should land on a 1s interval (between the old 2s and the 300ms event-driven ideal) — good enough for human-scale hot-reload without the platform-crate cost. If option (b), link the justification and close as `wontfix`.
