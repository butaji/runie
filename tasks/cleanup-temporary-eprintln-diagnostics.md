# Clean up temporary eprintln diagnostics

**Status**: done
**Milestone**: R7
**Category**: TUI / Debugging
**Priority**: P3

**Depends on**: none
**Blocks**: none

## Description

The files mentioned in the original task (`crates/runie-agent/src/actor.rs`, `crates/runie-agent/src/stream_response.rs`, `crates/runie-tui/src/ui_actor_agent_handles.rs`) do not contain any `eprintln!` calls. The temporary debug logging was either previously removed or never existed in those files.

## Verification

All remaining `eprintln!` calls in the codebase are:
- `crates/runie-cli/src/main.rs` — legitimate user-facing error output
- `crates/runie-core/src/tool/shim/mod.rs` (test functions) — debug aids for test failures
- `crates/runie-core/src/markdown/tests.rs` (test functions) — debug aids for test failures
- `crates/runie-core/build.rs` — build script output, not user-facing

## Acceptance Criteria

- [x] No `eprintln!` calls exist in `crates/runie-agent/src/actor.rs`, `crates/runie-agent/src/stream_response.rs`, or `crates/runie-tui/src/ui_actor_agent_handles.rs`.
- [x] Remaining `eprintln!` calls are either legitimate error output or test-only debug aids.
- [x] `cargo test --workspace` passes.
- [x] `cargo check --workspace` has no warnings.

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
> **Live tmux testing session required:** After the implementation passes unit and E2E tests, run a real terminal tmux session that exercises the changed behavior. The task is not done until the live session succeeds.
