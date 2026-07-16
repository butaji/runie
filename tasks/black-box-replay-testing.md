# Black-box replay testing for runie CLI and TUI

## What this is

The `runie-tests` crate runs the real `runie-cli` and `runie-tui` binaries against
recorded SSE fixtures. It has zero Rust crate dependency on `runie`: it locates
or builds the binaries, spawns them as subprocesses, and asserts on observable
output. No API keys or network calls are required at test runtime.

## Current implementation

| Decision | Actual state |
|---|---|
| Test suite location | `~/code/GitHub/runie-tests/` (sibling to `runie/`) |
| Dependency on runie code | Zero crate imports; only subprocess binaries |
| Binary discovery | `RUNIE_BIN` → `runie-tui`; `RUNIE_CLI_BIN` → `runie`; else build from `runie/` submodule. See `src/app_test.rs::find_runie_tui_binary()` and `src/app_test.rs::find_runie_cli_binary()`. |
| Build mode | Debug by default; `RUNIE_BUILD_MODE=release` selects release. `cargo build -p runie-cli -p runie-tui` is run once and cached. |
| Fixture location | `runie-tests/fixtures/{openai,anthropic}/` |
| Recording scripts | `runie-tests/scripts/record_opencode_go.py` and `record_opencode_go_multiturn.py` |
| TUI execution | Real `tmux` sessions via `tmux` CLI; pane capture via `tmux capture-pane -p -t <session>`; ANSI stripped with `strip_ansi_escapes::strip_str()`. |
| Completion detection | `wait_for_idle()` polls pane content until it is stable for one poll interval (50 ms) within `TimeoutConfig::idle` (1 s default). Text assertions use `wait_for_text()` which polls every 100 ms. |
| Assertions | Predicate-based: `contains`, `not_contains`, `matches`, `and`, `or`; exit-code predicates `Success` and `Failure`. |
| Environment isolation | Temp `HOME` with `~/.runie/config.toml`; `XDG_CONFIG_HOME`, `XDG_DATA_HOME`, `XDG_CACHE_HOME`, `XDG_STATE_HOME` unset; `RUNIE_TEST_DATA_DIR=<temp_home>`. |
| Fixture loading | `RUNIE_REPLAY_FIXTURES=<path1>,<path2>` passed to the child process. |
| Protocol selection | Auto-detected from fixture path (`fixtures/openai/` vs `fixtures/anthropic/`) or overridden with `RUNIE_REPLAY_PROTOCOL=openai\|anthropic`. |
| Fake tool outputs | Static registry inside `runie` replay provider. `get_weather` returns `{"temperature":22,"unit":"celsius","condition":"sunny"}`; `read_file` returns `{"path":"README.md","content":"# Example Project\n\nThis is a sample README.\n"}`; `list_dir` returns `{"entries":["README.md","Cargo.toml"]}` (fixture-specific values may differ). |
| Session reuse | Mock sessions are cached per test file for 5 minutes (`SESSION_REUSE_DURATION`) and reset with `/new` via `AppTest::reset_chat()`. |
| CI | None currently. |

## Files in runie-tests

```text
runie-tests/
├── Cargo.toml
├── src/
│   ├── lib.rs              # public re-exports and prelude
│   ├── cli.rs              # CliTest / CliAssert builders
│   ├── tui.rs              # TuiTest / TuiAssert builders
│   ├── fixtures.rs         # fixture_path() and fixture_path! macro
│   ├── predicates.rs       # Predicate trait and helpers
│   ├── keys.rs             # Key constants (CTRL_Q, ENTER, etc.)
│   ├── app_test.rs         # AppTest tmux harness and timeout config
│   ├── test_config.rs      # TestConfig / ProviderConfig builders
│   └── prelude.rs          # Convenience re-exports
├── tests/
│   ├── cli_replay.rs       # CLI replay tests
│   ├── replay_dsl_smoke.rs      # DSL smoke tests
│   ├── tui_replay_conversations.rs # TUI replay tests
│   └── ... other feature-specific tests
├── fixtures/
│   ├── openai/
│   └── anthropic/
└── scripts/
    ├── record_opencode_go.py
    └── record_opencode_go_multiturn.py
```

## Replay provider behavior

When `RUNIE_REPLAY_FIXTURES` is set, the `runie` provider factory:

1. Parses the comma-separated list of filesystem paths.
2. Detects protocol from path (`.../openai/...` vs `.../anthropic/...`) or from
   `RUNIE_REPLAY_PROTOCOL`.
3. Reads each `.sse` file into memory.
4. Constructs a `ReplayProvider` that replays fixtures round-robin.
5. Skips API-key/base-URL resolution.

For tool-call fixtures, the replay provider looks up the tool name in a static
registry and emits a matching `ToolResult` event instead of invoking a real tool.

## CLI replay DSL

Implemented in `src/cli.rs`. Entry point:

```rust
use runie_tests::prelude::*;

#[tokio::test]
async fn print_replays_simple_text() {
    test_cli()
        .fixture("openai/opencode_go_deepseek_v4_flash_simple.sse")
        .args(["print", "say ok"])
        .assert()
        .await
        .unwrap()
        .stdout(contains("ok"))
        .success();
}
```

Multi-turn CLI test:

```rust
#[tokio::test]
async fn weather_chain_replays() {
    test_cli()
        .fixtures([
            "openai/opencode_go_deepseek_v4_pro_multiturn_weather_chain_turn1.sse",
            "openai/opencode_go_deepseek_v4_pro_multiturn_weather_chain_turn2.sse",
        ])
        .args([
            "print",
            "What is the weather in Paris?",
            "What about Berlin?",
        ])
        .assert()
        .await
        .unwrap()
        .stdout(contains("Paris"))
        .success();
}
```

## TUI replay DSL

Implemented in `src/tui.rs`. Entry point:

```rust
use runie_tests::prelude::*;

#[tokio::test]
async fn tui_displays_streaming_text() {
    test_tui()
        .fixture("openai/opencode_go_kimi_k2_6_simple.sse")
        .type_keys("say ok")
        .submit()
        .wait_for_idle(Duration::from_millis(500))
        .capture_pane()
        .assert(contains("ok"));
}
```

## Coverage matrix

| Scenario | Protocol | Fixture example | Test file |
|---|---|---|---|
| Simple streaming text | openai | `opencode_go_deepseek_v4_flash_simple.sse` | `tests/cli_replay.rs` |
| Simple streaming text | anthropic | `opencode_go_minimax_m3_simple.sse` | `tests/cli_replay.rs` |
| Tool call | openai | `opencode_go_deepseek_v4_flash_tool.sse` | `tests/cli_replay.rs` |
| Reasoning | openai | `opencode_go_deepseek_v4_flash_reasoning.sse` | `tests/cli_replay.rs` |
| Multi-turn weather chain | openai | `opencode_go_deepseek_v4_flash_multiturn_weather_chain_turn{1,2}.sse` | `tests/cli_replay.rs` |
| Simple chat | openai | `opencode_go_kimi_k2_6_simple.sse` | `tests/tui_replay_conversations.rs` |
| Tool chat | anthropic | `opencode_go_minimax_m3_tool.sse` | `tests/tui_replay_conversations.rs` |
| Reasoning chat | openai | `opencode_go_deepseek_v4_flash_reasoning.sse` | `tests/tui_replay_conversations.rs` |
| Multi-turn chat | openai | `opencode_go_kimi_k2_6_multiturn_math_chain_turn{1,2}.sse` | `tests/tui_replay_conversations.rs` |

Full fixture coverage is tracked in `tasks/recorded-trace-coverage.md`.

## Acceptance checklist

- [x] `runie-tests` crate builds independently.
- [x] `cargo test --test cli_replay` builds runie from `runie/` and passes.
- [x] `cargo test --test tui_replay_conversations` passes for the TUI replay matrix.
- [x] CLI print/json tests pass for all fixtures in the matrix.
- [x] No API keys or network calls are required for existing tests.
- [x] Tests are deterministic on repeated runs.
- [x] No CI is added.
