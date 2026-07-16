# Unify replay TUI harness with AppTest

## Objective

Eliminate the duplicated tmux session startup, key sending, and wait logic
between `src/tui.rs` (`TuiTest`) and `src/app_test.rs` (`AppTest`). Replay TUI
tests should reuse the same session management and timeout infrastructure as
mock TUI tests.

## Why this matters

`TuiTest` and `AppTest` both managed tmux sessions, sent keys, captured pane
text, and waited for UI state. Keeping two harnesses:

- doubled the surface area for timeout and polling bugs,
- prevented replay tests from benefiting from session-reuse optimizations,
- made the suite harder to maintain as production quality.

## Implementation

Completed in branch `speedup-phase1-unify-replay-harness`.

1. Extended `AppTest` to support replay mode:
   - Added `AppMode::Replay`.
   - Added `replay_fixtures` and `replay_protocol` fields.
   - Added `AppTest::replay(fixture)` and
     `AppTest::replay_with_fixtures(fixtures)` constructors.
   - Added `AppTest::replay_protocol(protocol)` setter.
   - `AppTest::start()` writes `TestConfig::replay()` and sets
     `RUNIE_REPLAY_FIXTURES` / `RUNIE_REPLAY_PROTOCOL` on the tmux session.
   - Added `AppTest::send_raw(keys)` for raw tmux key sequences.
   - Added `AppTest::wait_for_idle_with_stability(stability, max_wait)`.

2. Refactored `src/tui.rs` so `TuiTest` delegates to `AppTest`:
   - Removed duplicated tmux session creation, key sending, capture, and idle
     wait logic.
   - Added a static per-test-file cache keyed by
     `(test_file, fixture, protocol, pid)` for single-fixture tests.
   - `capture_pane()` gets or creates a cached `AppTest`, holds the mutex for
     the whole test body (reset, send keys, idle wait, capture), and returns
     `TuiAssert`.
   - Multi-fixture tests still create a fresh session because fixtures are
     consumed per session.
   - Preserved the original `test_tui()` public API and `TuiAssert`
     assertions.

3. Verified:
   - `cargo test --test tui_replay_conversations` passes (25/25).
   - `cargo test --test error_recovery` passes (12/12).
   - `cargo test --test error_state_rendering` passes (15/15).
   - Focused full-suite run showed no regressions in replay or mock tests.

## Acceptance checklist

- [x] No duplicated tmux session startup code remains in `src/tui.rs`.
- [x] `TuiTest` uses the same `TimeoutConfig`, polling, and `wait_for_idle`
      implementation as `AppTest`.
- [x] Single-fixture replay TUI tests share a session per test file.
- [x] All replay TUI tests pass with default cargo parallelism.
- [x] Wall-clock time for `tui_replay_conversations` is reduced.
- [x] `tasks/improve-test-execution-speed.md` is updated to mark the harness
      unification opportunity as done.

## Dependencies

- `black_box_replay_dsl` (done)
- `tui_replay_conversations` (done)
- `tui_dsl_polling_waits` (done)
