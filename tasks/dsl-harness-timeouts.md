# DSL harness timeouts

## Objective

Add a unified, harness-level timeout layer to the black-box DSL so no test can
hang forever. Every blocking operation — tmux session start, key send, pane
capture, text wait, binary build, and CLI process wait — must have a single
configurable default timeout and a clear error path.

## Why this matters

Tests currently get stuck in `capture_pane`, `expect_text`, and command waits
because the DSL has no global timeout policy. This wastes CI time, requires
manual kills, and hides whether the app is slow or genuinely deadlocked. A
unified harness timeout stops tests reliably and reports exactly which operation
failed.

## Unified timeout model

Introduce one `Timeout` configuration object used by every DSL operation:

```rust
pub struct TimeoutConfig {
    pub startup: Duration,       // tmux session spawn, runie-tui launch
    pub keystroke: Duration,     // small pause between keys, if needed
    pub response: Duration,      // wait_for_text / expect_text
    pub idle: Duration,          // wait_for_idle / capture_pane stability
    pub dialog: Duration,        // permission/model/command pickers
    pub build: Duration,         // cargo build of the runie submodule
    pub cli: Duration,           // runie-cli process exit wait
}

impl Default for TimeoutConfig {
    fn default() -> Self {
        Self {
            startup: Duration::from_secs(5),
            keystroke: Duration::from_millis(10),
            response: Duration::from_secs(10),
            idle: Duration::from_secs(3),
            dialog: Duration::from_secs(3),
            build: Duration::from_secs(300),
            cli: Duration::from_secs(10),
        }
    }
}
```

Expose it through the builders:

```rust
AppTest::mock()
    .timeout(TimeoutConfig { response: Duration::from_secs(20), ..Default::default() })
    .start().await?
    .expect_text("ok").await?;
```

## Operations that must be wrapped

1. **`AppTest::start()`** — timeout on tmux session creation and first pane
   capture.
2. **`press()` / `type_text()`** — timeout if tmux does not accept input.
3. **`capture_pane()`** — timeout on pane read and on idle/stability wait.
4. **`expect_text()` / `expect_no_text()`** — timeout on the polling loop.
5. **`wait_for_idle()`** — timeout if pane never stabilizes.
6. **`ensure_runie_binaries_built()`** — timeout on cargo build.
7. **`tokio::process::Command` waits in CLI DSL** — timeout on process exit.

## Error behavior

On timeout:

- Kill the tmux session and any spawned process.
- Capture the last known pane content (if available).
- Return a structured error:

```rust
Err(anyhow!(
    "timeout waiting for {:?} after {:?}\nlast pane:\n{}",
    operation, timeout, last_pane
))
```

This replaces panics, infinite loops, and hung CI jobs with actionable failures.

## Consolidation

This task supersedes the scattered timeout handling addressed by:

- `centralize-test-timeouts` — constants become fields of `TimeoutConfig`.
- `tui-dsl-polling-waits` — polling waits use `TimeoutConfig::response` and
  `TimeoutConfig::idle`.
- `remove-test-sleep` — sleeps become waits with timeouts.

## Dependencies

- `black_box_replay_dsl`

## Acceptance checklist

- [x] A single `TimeoutConfig` struct exists and is used by all DSL internals.
- [x] Every blocking DSL operation has a default timeout and an override path
      via `AppTest::with_timeout()`.
- [x] Tight defaults are set (`startup: 5s`, `response: 10s`, `idle: 1s`,
      `dialog: 3s`, `build: 300s`) and `TEST_TIMEOUT` is 60s.
- [x] A test that intentionally hangs is killed and reports the operation that
      timed out.
- [x] No test can hang forever; the longest possible wait is `TimeoutConfig::build`.
- [x] Timeout errors include the last captured pane content for debugging.
- [x] `AGENTS.md` documents `with_timeout()` and the performance principles.
