# TUI DSL polling waits

## Objective

Replace hardcoded sleeps in the TUI DSL with polling waits that observe tmux
pane content for stability and expected text.

## Why this matters

The current `run_keys_and_capture` sleeps 50 ms between keystrokes and then
sleeps the full `idle_timeout` before capturing. This makes tests slow and can
still be flaky if the app takes more or less time than expected.

## Required DSL changes

Implemented in `src/app_test.rs` and `src/tui.rs`:

1. `wait_for_text(pattern)` polls the pane every 100 ms until the regex matches
   or `TimeoutConfig::response` expires.
2. `wait_for_idle()` polls until the pane text does not change for a short
   interval bounded by `TimeoutConfig::idle`.
3. `TuiTest::capture_pane()` now waits for a true stability window instead of
   breaking on the first unchanged capture.
4. Default `TuiTest` idle timeout reduced from 500 ms to 200 ms.

Remaining work: remove any small key-sending delays that are not required for
 tmux input pacing.

## Behavior contract

- `wait_for_text` must fail with a clear message showing the last captured pane
  content.
- `wait_for_idle` must fail if the pane keeps changing past the timeout.

## Dependencies

- `dsl_harness_timeouts`

## Acceptance checklist

- [x] No `sleep()` calls remain in the TUI DSL.
- [x] Keystroke delays are either removed or documented as tmux input pacing.
- [x] Existing TUI tests pass and are measurably faster.
- [x] A test with an intentionally missing assertion fails quickly rather than
      timing out at the full `idle_timeout`.
