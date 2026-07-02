# Move blocking IO out of command/update handlers

**Status**: done
**Milestone**: R7
**Category**: Architecture / Actors
**Priority**: P1

**Depends on**: remove-direct-appstate-mutation-from-tui-handlers
**Blocks**: none

## Description

Trust loading, session metadata loading, and a bash fallback run synchronously inside command/update handlers that execute on the async event loop. This can freeze the TUI when `/session_info`, `/resume`, `/sessions`, or `!bash` are used.

## Root Cause

Legacy code paths in `crates/runie-core/src/commands/dsl/handlers/session/mod.rs` and `crates/runie-core/src/update/input/submit.rs` call synchronous file IO directly.

## Acceptance Criteria

- [ ] No synchronous file IO runs in command/update handlers.
- [ ] Trust/session metadata queries are sent to `IoActor` / `SessionActor` and results applied via facts.
- [ ] The bash fallback in `submit.rs` is removed or moved to a non-blocking tool path.
- [ ] `cargo test --workspace` passes.
- [ ] Live tmux `/session_info` and `/sessions` remain responsive.

## Tests

### Layer 1 — State/Logic
- [ ] `session_info_handler_sends_io_intent` — `/session_info` emits a `SessionMsg::Info` or `IoMsg` instead of reading disk.

### Layer 2 — Event Handling
- [ ] `session_info_result_renders` — the fact returned by `SessionActor` is rendered as a message.

### Layer 4 — Provider Replay / Mock-Tool E2E
- [ ] `tmux_session_info_responsive` — live tmux script runs `/session_info` and asserts no UI freeze.

## Files touched

- `crates/runie-core/src/commands/dsl/handlers/session/mod.rs`
- `crates/runie-core/src/update/input/submit.rs`
- `crates/runie-core/src/actors/io/ractor_io.rs`
- `crates/runie-core/src/actors/session/ractor_session_actor.rs`

## Validation

This task is not complete until the fix is validated with all three levels:

1. **Unit tests** — cover the state/logic change in isolation.
2. **E2E tests** — cover the event handling and/or provider-replay path.
3. **Live tmux tests** — `scripts/tmux-smoke-test.sh mock` (or the relevant scenario) passes in a real terminal.

## Notes

- This is part of the broader “no blocking work in handlers” architecture rule.
> **Live tmux testing session required:** After the implementation passes unit and E2E tests, run a real terminal tmux session that exercises the changed behavior. The task is not done until the live session succeeds.
