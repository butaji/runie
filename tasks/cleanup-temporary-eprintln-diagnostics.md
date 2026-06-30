# Clean up temporary eprintln diagnostics

**Status**: todo
**Milestone**: R7
**Category**: TUI / Debugging
**Priority**: P3

**Depends on**: none
**Blocks**: none

## Description

Temporary `eprintln!` diagnostics are present in the working tree. If committed, they will clutter user terminals and slow the event loop.

## Root Cause

Debug logging added during the mock hello repetition investigation was not removed.

## Acceptance Criteria

- [ ] All temporary `eprintln!` calls in `crates/runie-agent/src/actor.rs`, `crates/runie-agent/src/stream_response.rs`, and `crates/runie-tui/src/ui_actor_agent_handles.rs` are removed.
- [ ] If persistent logging is needed, replace with `tracing` at the appropriate level.
- [ ] `cargo test --workspace` passes.
- [ ] `cargo check --workspace` has no new warnings.

## Files touched

- `crates/runie-agent/src/actor.rs`
- `crates/runie-agent/src/stream_response.rs`
- `crates/runie-tui/src/ui_actor_agent_handles.rs`

## Validation

This task is not complete until the fix is validated with all three levels:

1. **Unit tests** — cover the state/logic change in isolation.
2. **E2E tests** — cover the event handling and/or provider-replay path.
3. **Live tmux tests** — `scripts/tmux-smoke-test.sh mock` (or the relevant scenario) passes in a real terminal.

## Notes

- This should be done as part of the first functional fix or in a dedicated cleanup commit before any release.
