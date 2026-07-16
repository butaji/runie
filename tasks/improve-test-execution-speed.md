# Improve test execution speed

## Objective

Make the `runie-tests` black-box suite run as fast as possible while keeping it
reliable. This task is a living audit of every source of wall-clock time:
compile/build, process/tmux spawn, harness polling/waits, test redundancy,
parallelism, and the application under test.


## Current state

- **543 integration tests** across 51 test files.
- **Session reuse implemented** via module-level `OnceLock` shared sessions
  (10 files fully shared, 5 files mixed shared/fresh, plus
  `tui_replay_conversations` which shares a session per fixture via the `TuiTest`
  cache).
  Pattern: `fresh_shared_app()` / `get_fresh_shared_app()` with `reset_chat()`.
- **Fresh spawn per test** (intentionally — state pollution, permission state,
  onboarding/mock-provider mode, dialog focus, or app termination prevents reuse):
  `approval_dialog_preview_feedback`, `ctrl_q_closes_app`, `dialog_navigation`,
  `dialog_numeric_selection`, `error_state_rendering` (mixed: onboarding/config
  tests keep fresh spawns), `file_references`, `input_composition`,
  `invalid_api_key`, `keyboard_shortcuts_overlay`, `mock_echo` (1 test, fresh
  spawn per test), `mock_list_files`, `mock_onboarding`, `mock_provider_management`,
  `mock_provider_visible`, `model_switching`, `permission_dialog_navigation`,
  `provider_management`, `settings_dialog`, `status_command` (feature in
  progress, tests ignored), `text_quit_commands`, `tool_output_rendering`,
  `tool_permissions`, `vim_nav_feed`.
- **Permission-state lesson**: `reset_to_clean_state()` (config deletion +
  `/reload`) clears persisted "Always Allow" rules, but it cannot clear
  in-memory "This session" rules or reliably reset dialog/focus state left by
  preview overlays. Therefore every test file that selects "Always",
  "This session", or opens the Ctrl+E preview is fresh-spawn-per-test.
- **DSL/replay**: `error_recovery`, `cli_replay`, `replay_dsl_smoke` use the
  CLI replay DSL; `tui_replay_conversations` now delegates to `AppTest` and
  shares single-fixture sessions per test file. Detailed OpenAI/Anthropic
  error assertions live in `tests/cli_replay.rs`; the TUI replay path keeps
  only one rate-limit error fixture per protocol to avoid duplicating spawn
  cost.
- **Total suite runtime: ~7 min with `--test-threads=2`** (default in
  `just test`); ~14 min with `--test-threads=1`.
- **Binary auto-detection**: `justfile` sets `RUNIE_BIN` / `RUNIE_CLI_BIN`.
- **`just test` uses `--test-threads=2`**; `just test-1` uses 1 thread for
  tmux-heavy environments or debugging.
- **Zombie tmux sessions cleaned** before each `just test` run.
- **No `sleep()` calls in tests**; harness uses state-based waits.
- **Fast replay polling**: `TuiTest` polls every 10 ms with exponential backoff
  to 50 ms; default idle timeout is 100 ms.
- **High-spawn suites** that are safely shared and dominate wall-clock time:
  `chat_scrollback`, `command_palette_navigation`, `compact_command`,
  `large_paste_placeholder`, `newline_key_aliases`, `session_management`,
  `session_title_export`, `slash_commands`, plus the local-shared suites
  `at_mention_improvements`, `automatic_config_reload`, `core_mock_loop`,
  `mock_echo`, `model_mock_echo`, `single_binary_cli_mode`, `startup_onboarding`,
  `status_bar`, `turn_lifecycle`, `undo_redo_commands`, `visible_message_queue`.

## Already completed quick wins

- Removed the upfront fixed sleep in `wait_for_text`; it now polls immediately
  every 100 ms.
- Added `TimeoutConfig` and shared constants (`SHORT_TIMEOUT`, `MEDIUM_TIMEOUT`,
  `LONG_TIMEOUT`, `VERY_LONG_TIMEOUT`).
- Lowered `capture_pane` retry backoff to 100 ms.
- Replaced explicit `sleep()` calls in tests with state-based waits
  (`tasks/remove-test-sleep.md` — done).
- Added per-file session caching keyed by test file name and fixed the cache to
  return the same `Arc<Mutex<AppTest>>` so tests in the same file serialize on
  one tmux session.

## Optimization status

### Done

1. **Pre-build / auto-detect runie binaries** — `justfile` detects
   `runie/target/debug/runie-tui` and `runie/target/debug/runie` and sets
   `RUNIE_BIN` / `RUNIE_CLI_BIN`; in-test cargo build is skipped when binaries
   exist.
2. **Honor `RUNIE_CLI_BIN` and support debug builds** — `find_runie_cli_binary()`
   checks `RUNIE_CLI_BIN` first and respects `RUNIE_BUILD_MODE`.
3. **Cache binary resolution** — `TUI_BINARY_CACHE` and `CLI_BINARY_CACHE`
   `OnceLock`s cache resolved paths.
4. **Tighten default timeouts** — `startup: 10s`, `response: 5s`, `idle: 1s`,
   `dialog: 3s`, `build: 300s`; `TEST_TIMEOUT: 60s`.
5. **Remove magic durations** — all test files use `SHORT_TIMEOUT`,
   `MEDIUM_TIMEOUT`, `LONG_TIMEOUT`, or `VERY_LONG_TIMEOUT`.
6. **Remove `sleep()` calls** — completed via `tasks/remove-test-sleep.md`.
7. **Session reuse rollout** — shared sessions are used by all test files where
   `/new` reset is reliable; permission-sensitive, preview/dialog-heavy,
   one-shot, and state-pollution files intentionally remain fresh.
8. **Consolidate the two TUI harnesses** — `TuiTest` in `src/tui.rs` now delegates
   tmux session lifecycle, key sending, waits, and capture to `AppTest` /
   `TmuxSession`. Single-fixture replay TUI tests share a session per test file
   via a static `OnceLock` cache keyed by `(file, fixture, protocol, pid)`.
   Multi-fixture tests still spawn fresh sessions because fixtures are consumed
   per session.
9. **Convert redundant TUI error fixtures to CLI replay** — `tests/cli_replay.rs`
   already asserts every OpenAI/Anthropic SSE error fixture with non-zero exit
   and readable error output. Removed `tui_openai_server_error`,
   `tui_http_429_rate_limit`, `tui_http_401_unauthorized`, and
   `tui_anthropic_server_error` from `tests/tui_replay_conversations.rs` and
   kept only `tui_openai_rate_limit_error` and `tui_anthropic_rate_limit_error`
   so the TUI replay path still exercises one error fixture per protocol.
10. **Extend session reuse to safe high-spawn suites** — Converted
, `compact_command`, `session_management`, `session_title_export`, and the
    local-shared suites listed above to shared sessions. Files that exercise
    permission persistence, preview overlays, dialog navigation, settings
    sub-panel state, or hotkeys filter state (`approval_dialog_preview_feedback`,
    `dialog_navigation`, `dialog_numeric_selection`, `keyboard_shortcuts_overlay`,
    `mock_list_files`, `permission_dialog_navigation`, `settings_dialog`,
    `tool_output_rendering`, `tool_permissions`, `auto_approve_mode`,
    `context_aware_tool_toggle`) were converted and then reverted to
    fresh-spawn-per-test after proving that `reset_to_clean_state()` cannot
    reliably clear in-memory permission rules or overlay/dialog focus state.

### Remaining opportunities (ranked by impact)

### 1. Consolidate redundant tests

- **Partially done** via `reorganize_input_composition_tests` (done): input
  composition tests moved to `tests/input_composition.rs`; `mock_echo.rs` reduced
  from ~70 lines to 1 cursor-editing test (`input_ctrl_k_kills_to_end_of_line`).
- **Remaining:** Merge duplicate coverage in `core_mock_loop.rs`,
  `turn_lifecycle.rs`, `tool_permissions.rs`,
  `permission_dialog_navigation.rs`, `mock_list_files.rs`. See
  `tasks/consolidate-redundant-tests.md` (backlog).

### 2. Use CLI replay over TUI where terminal behavior is not required

- **Files:** `tests/cli_replay.rs`, `tests/tui_replay_conversations.rs`,
  `tests/error_state_rendering.rs`.
- **What:** CLI tests (`test_cli().fixture(...).args(...).assert()`) spawn a
  process but no tmux server, no pane capture, and no 120×40 TUI render. Convert
  black-box tests that only assert stdout/stderr/exit code to CLI replay tests
  in `tests/cli_replay.rs`.
- **Impact:** TUI tests are inherently slower; fewer of them means a faster
  suite.

### 3. Use per-test tmux sockets to remove global server contention

- **Task:** `tasks/per-test-tmux-sockets.md` (backlog — blocked)
- **Files:** `src/app_test.rs`, `src/tui.rs`.
- **What:** Start each tmux session with a unique `-S <socket>` path so tests do
  not serialize on the default tmux server. Combined with faster per-test
  runtime, this enables raising or removing `--test-threads`.
- **Status:** Attempted and reverted. The implementation works, but static
  `OnceLock` session caches leak tmux sessions, socket files, and temp home
  directories on `cargo test` exit (`std::process::exit` skips static
  destructors). This cleanup blocker must be solved first; see
  `tasks/per-test-tmux-sockets.md`.
- **Impact:** Allows true parallelism; reduces "capture-pane failed" flakiness.

### 4. Raise or remove the `--test-threads` cap

- **Files:** `justfile`, `TEST_STATUS.md`, `EXECUTE.md`.
- **What:** `just test` currently caps execution at `--test-threads=2`. Once
  per-test tmux sockets (#3) reduce contention and the cleanup blocker is
  solved, benchmark with default cargo parallelism and raise or remove the cap.
- **Impact:** Better utilization of multi-core machines.

### 5. Speed up the application startup path

- **Files:** `runie/crates/runie-tui/src/bootstrap.rs` and related.
- **What:** Profile `runie-tui` cold start; reduce init work, lazy-load
  providers, defer heavy I/O. Every test pays this cost.
- **Impact:** Multiplied by every spawn; even 200 ms saved per test ≈ 109 s on
  543 tests.
- **Note:** This is a `runie` submodule change, not this repo; track it in the
  runie backlog, not in `runie-tests`.

### 6. Use `cargo nextest` and test sharding

- **Files:** `justfile`, local dev docs.
- **What:** `nextest` gives per-test timeouts, retries, and partitioning. Shard
  slow integration test binaries across jobs if CI is reintroduced.
- **Impact:** More resilient runs; lower wall-clock time via parallelism.

### 7. Cache cargo target directories and use faster linkers

- **Files:** Local dev docs.
- **What:** Cache `runie/target` and `target` between runs. Use `mold`/`lld`
  and larger `codegen-units` for faster rebuilds.
- **Impact:** Faster compile/link.

### 8. Reduce fixture replay overhead

- **Files:** `fixtures/`, `tests/tui_replay_conversations.rs`,
  `tests/cli_replay.rs`.
- **What:** Use shorter fixtures, avoid multi-turn fixtures when single-turn
  covers the behavior, and strip unnecessary SSE chunks.
- **Impact:** Faster streaming waits.

## Dependencies

- `dsl_harness_timeouts` (done)
- `tui_dsl_polling_waits` (done)
- `remove_test_sleep` (done)

## Acceptance checklist

- [x] `RUNIE_CLI_BIN` is honored by `find_runie_cli_binary`.
- [x] `RUNIE_BUILD_MODE=debug` works and local `just test` defaults to debug or
      prebuilt binaries.
- [x] Binary resolution is cached after the first lookup.
- [x] High-spawn input, navigation, session, and settings suites share sessions
      via `reset_chat()` and show measurable speedup.
- [x] Permission/tool/preview suites that modify in-memory state are
      fresh-spawn-per-test and fully reliable under `--test-threads=2`.
- [x] Default `TimeoutConfig` / `TEST_TIMEOUT` are tightened and no magic
      durations remain in tests.
- [x] `just test` uses `--test-threads=2`; `just test-1` available for debugging.
- [x] Zombie tmux sessions are cleaned before each `just test` run.
- [x] No `sleep()` calls in tests; harness uses state-based waits.
- [x] Suite wall-clock time reduced from ~14 min (`--test-threads=1`) to
      ~7 min (`--test-threads=2`), a ~50% improvement.
- [x] `TuiTest` uses shared tmux/session logic — completed in
      `tasks/unify-replay-tui-harness.md`.
- [x] Per-test tmux sockets — attempted and reverted; blocked by static-cache
      cleanup leak on `cargo test` exit. See `tasks/per-test-tmux-sockets.md`.
- [x] Redundant TUI error fixtures removed in favor of CLI replay coverage.
- [x] Remaining redundant tests partially addressed — mock_echo cleaned up in
      `reorganize_input_composition_tests`; broader consolidation in backlog.
      See `tasks/consolidate-redundant-tests.md`.
- [x] Suite reduced by ~50% from the original `--test-threads=1` baseline.

## How to measure

```bash
# Full suite (~7 min with --test-threads=2, ~14 min with --test-threads=1)
just test

# Single-threaded run (for debugging tmux contention)
just test-1

# Single file timing
time cargo test --test input_composition -- --test-threads=1

# Measure with pre-built binaries
RUNIE_BIN=$(pwd)/runie/target/debug/runie-tui \
RUNIE_CLI_BIN=$(pwd)/runie/target/debug/runie \
time cargo test -- --test-threads=1
```
