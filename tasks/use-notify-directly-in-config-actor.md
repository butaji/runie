# Use `notify` directly in `RactorConfigActor`

**Status**: todo
**Milestone**: R2
**Category**: Configuration / Actors
**Priority**: P1

**Depends on**: route-cli-config-through-configactor
**Blocks**: none

## Description

`crates/runie-core/src/actors/config/ractor_config.rs` spawns a `std::thread` that runs the `notify` debouncer and forwards `ConfigMsg::Reload` over an mpsc channel, plus a tokio task that loops on that channel. `notify` is already a dependency. The debouncer can be created in `pre_start` and its closure can send directly to the actor’s `ActorRef`, removing the thread bridge and ~80 lines.

## Acceptance Criteria

- [ ] Remove the custom `spawn_watcher`/`spawn_watcher_task` and `block_watcher_loop` helpers.
- [ ] Create the `notify` debouncer in `RactorConfigActor::pre_start` with a closure that calls `actor_ref.cast(ConfigMsg::Reload)`.
- [ ] Preserve debounce timing and error handling.
- [ ] Config file changes still trigger a reload in tests.
- [ ] `cargo test --workspace` succeeds after the change.
- [ ] `cargo check --workspace` succeeds with no new warnings.

## Tests

### Layer 1 — State/Logic
- [ ] `config_reload_on_file_change` — write to the config file and assert `RactorConfigActor` reloads.
- [ ] `no_mpsc_bridge_remains` — `ractor_config.rs` no longer contains a `std::sync::mpsc` or `tokio::sync::mpsc` watcher bridge.

### Layer 2 — Event Handling
- [ ] N/A.

### Layer 3 — Rendering
- [ ] N/A.

### Layer 4 — Provider Replay / Mock-Tool E2E
- [ ] N/A.

## Files touched

- `crates/runie-core/src/actors/config/ractor_config.rs`
- `crates/runie-core/src/actors/config/file_helpers.rs`
- `crates/runie-core/src/actors/config/messages.rs`

## Notes

- Depends on `route-cli-config-through-configactor.md` because the watcher wiring is part of the same actor refactor.
- `notify` is already in `Cargo.toml`; no new dependency is needed.
