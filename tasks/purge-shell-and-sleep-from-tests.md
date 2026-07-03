# Purge shell tests, `sleep()` calls, and tmux smoke script from automatic tests

**Status**: todo
**Milestone**: R7
**Category**: Test harness
**Priority**: P1

**Depends on**: none
**Blocks**: none

## Description

The codebase still contains several testing anti-patterns that `AGENTS.md` forbids:

1. `crates/runie-core/src/actors/io/ractor_io/tests.rs` shells out to `echo`, `pwd`, `git`, and `sh`.
2. Multiple test files use `tokio::time::sleep` for synchronization:
   - `crates/runie-core/src/tool/cache.rs:380`
   - `crates/runie-core/src/actors/turn/tests.rs:245, 288`
   - `crates/runie-agent/src/actor/tests.rs:120`
   - `crates/runie-agent/src/tests/permissions.rs:137`
   - `crates/runie-tui/src/bootstrap.rs:499`
   - `crates/runie-tui/src/tests/agent_run_guard.rs:315`
   - `crates/runie-tui/src/tests/input_actor_routing.rs:73, 148`
   - `crates/runie-tui/src/tests/uiactor_init.rs:85, 165`
3. `scripts/tmux-smoke-test.sh` still exists and uses tmux + wall-clock sleeps.

## Acceptance Criteria

- [ ] Replace shell subprocess tests in `ractor_io/tests.rs` with deterministic mock IO.
- [ ] Replace every `tokio::time::sleep` in automatic tests with event-driven synchronization or `tokio::time::pause`.
- [ ] Delete `scripts/tmux-smoke-test.sh` and remove any references in docs/recipes.
- [ ] `cargo test --workspace` passes.
- [ ] `cargo check --workspace` passes with no new warnings.

## Tests

### Layer 1 — State/Logic
- [ ] `fake_command_runner_returns_shell_result` — in-memory runner produces deterministic `ShellResult`s.
- [ ] `cache_sweep_evicts_without_wall_clock_sleep` — cache eviction test uses paused/advancing time.

### Layer 2 — Event Handling
- [ ] `wait_for_event_replaces_sleep` — helper waits for expected events with a deterministic timeout.

### Layer 3 — Rendering
- [ ] N/A — no rendering change.

### Layer 4 — Provider Replay / Mock-Tool E2E
- [ ] `io_actor_tests_run_without_shell` — existing IO actor behavior is preserved without host binaries.

### Live Tmux Testing Session
- [ ] N/A — this task removes tmux/shell tests.

## Files touched

- `crates/runie-core/src/actors/io/ractor_io/tests.rs`
- `crates/runie-core/src/tool/cache.rs`
- `crates/runie-core/src/actors/turn/tests.rs`
- `crates/runie-agent/src/actor/tests.rs`
- `crates/runie-agent/src/tests/permissions.rs`
- `crates/runie-tui/src/bootstrap.rs`
- `crates/runie-tui/src/tests/agent_run_guard.rs`
- `crates/runie-tui/src/tests/input_actor_routing.rs`
- `crates/runie-tui/src/tests/uiactor_init.rs`
- `scripts/tmux-smoke-test.sh`
- `justfile`
- `.github/workflows/ci.yml`

## Notes

- Supersedes the remaining work from `remove-sleep-from-automatic-tests.md` and `replace-tmux-test-with-ratatui-tests.md`.
- The `wait_for_event` helper from `remove-sleep-from-automatic-tests.md` can be reused.
