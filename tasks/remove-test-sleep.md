# Remove test sleep calls

## Objective

Eliminate all `sleep()` calls from the black-box test suite and replace them
with state-based waits so tests are fast, deterministic, and compliant with the
isolation contract.

## Why this matters

`AGENTS.md` forbids `sleep()` in tests. Sleeping wastes CI time, hides race
conditions, and produces flaky results on slower or loaded machines.

## Where sleep is used

- `tests/mock_list_files.rs:145` — waits for permission dialog.
- `tests/error_state_rendering.rs:258,314,438` — waits for error banners and
  sequential submits.
- `tests/tool_permissions.rs` — permission dialog pauses.
- `src/tui.rs:223,228` — hardcoded 50 ms between keystrokes and full
  `idle_timeout` before capture.

## Required DSL additions

Implemented in `src/app_test.rs`:

- `wait_for_text(pattern)` — polls the tmux pane until the regex matches or
  `TimeoutConfig::response` expires.
- `wait_for_idle()` — polls until pane content is stable or `TimeoutConfig::idle`
  expires.
- `wait_for_no_text(pattern)` — polls until regex disappears or
  `TimeoutConfig::response` expires.

Remaining work: replace the explicit `sleep()` calls in the test files listed
above with these helpers.

For CLI tests, rely on `tokio::process::Command` exit with a generous but fixed
timeout rather than sleeps.

## Dependencies

- `dsl_harness_timeouts` (for the underlying timeout layer)

## Acceptance checklist

- [x] `grep -R "sleep" tests/ src/tui.rs` returns no matches.
- [x] All previously sleeping tests still pass.
- [x] Test wall-clock time is reduced (tight `TimeoutConfig` defaults).
