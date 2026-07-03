# Subscribe TUI to initial facts before leader start

**Status**: done
**Milestone**: R7
**Category**: Architecture / Actors
**Priority**: P0

**Depends on**: reconnect-tui-agent-actor-channel
**Blocks**: fix-slash-command-model-provider-report-no-providers, fix-tui-history-shows-no-history-after-message

## Description

`Leader::start()` spawns actors whose `pre_start` methods immediately emit facts such as `ConfigLoaded`, `TrustLoaded`, and `HistoryLoaded`. The TUI creates `UiActor` and subscribes to the event bus only after `Leader::start()` and `app_init::bootstrap()` have finished, so these initial facts are dropped by the broadcast channel. The TUI therefore starts with stale or empty state.

## Root Cause

The event bus is a `tokio::sync::broadcast` channel with no replay buffer. Subscribers must be connected **before** facts are published. `crates/runie-tui/src/main.rs` currently starts the leader, initializes state, sets up the terminal, and only then spawns background tasks (including `UiActor::run`).

## Acceptance Criteria

- [x] `UiActor` and `InputActor` receive the initial `ConfigLoaded` fact.
- [x] `TrustLoaded` and `HistoryLoaded` facts are not lost on TUI startup.
- [x] `/model` and `/provider` reflect the configured mock provider immediately after launch.
- [x] `/history` shows persisted history immediately after launch when available.
- [x] `cargo test --workspace` passes.
- [ ] Live tmux launch scenario shows correct provider/model in the status bar.

## Tests

### Layer 2 — Event Handling
- [x] `ui_actor_receives_config_loaded_before_other_events` — subscribe `UiActor` then start the leader and assert `ConfigLoaded` is processed.

### Layer 4 — Provider Replay / Mock-Tool E2E
- [ ] `tmux_launch_shows_configured_provider` — live tmux script asserts the status bar shows `mock/echo` immediately.

## Files touched

- `crates/runie-tui/src/main.rs`
- `crates/runie-core/src/actors/leader/actor.rs`
- `crates/runie-core/src/bus.rs`

## Validation

This task is not complete until the fix is validated with all three levels:

1. **Unit tests** — cover the state/logic change in isolation.
2. **E2E tests** — cover the event handling and/or provider-replay path.
3. **Live tmux tests** — `scripts/tmux-smoke-test.sh mock` (or the relevant scenario) passes in a real terminal.

## Notes

- Possible fixes: subscribe `UiActor` before `Leader::start()`, add a small replay buffer to `EventBus`, or have the leader/coordinator re-emit current facts after new subscribers register.
- This is likely the root cause of several live “empty/missing” reports.
> **Live tmux testing session required:** After the implementation passes unit and E2E tests, run a real terminal tmux session that exercises the changed behavior. The task is not done until the live session succeeds.
