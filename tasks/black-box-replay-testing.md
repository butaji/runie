# Black-box replay testing for runie CLI and TUI

## Goal

Create a standalone black-box test suite that runs the real `runie-cli` and
`runie-tui` binaries against recorded SSE fixtures without any code dependency
on runie. The suite lives in `~/code/GitHub/runie-tests/` and builds runie
from `../runie` source as a pre-test step.

## Architecture decisions

| Decision | Choice |
|----------|--------|
| Test suite location | `~/code/GitHub/runie-tests/` (sibling to `runie/`) |
| Dependency on runie code | Zero — only subprocess binaries |
| How binaries are obtained | `cargo build` in `../runie` before tests run |
| Fixture location | `runie-tests/fixtures/{openai,anthropic}/` |
| Recording scripts | `runie-tests/scripts/` |
| TUI execution | Real `tmux` sessions via tmux CLI + pane capture |
| Completion detection | Wait for idle time + sentinel text |
| Assertions | Predicate-based (contains / not-contains / exit code) |
| Environment isolation | Temp `HOME` with generated `~/.runie/config.toml` |
| Fixture loading | `RUNIE_REPLAY_FIXTURES=path1,path2` |
| Fake tool outputs | Static registry by tool name |
| CI | None for now |

## Phases

### Phase 1 — Core smoke suite

- CLI `print` and `json` commands with single-turn fixtures.
- TUI basic chat turn with `TestBackend`-equivalent in-process path AND tmux
  path for terminal fidelity.
- Tool-call and reasoning fixture coverage.
- Multi-turn chain support for CLI and TUI.

### Phase 2 — Richer TUI rendering

- TUI tool-call rendering (tool name, args, result blocks).
- TUI reasoning/thinking block rendering.
- TUI error-state rendering using error fixtures.

### Phase 3 — Advanced interactions

- Model/provider selection UI.
- Slash commands.
- Settings dialogs and edge-case interactions.

This task focuses on Phase 1.

## Files to create in runie-tests

```text
runie-tests/
├── Cargo.toml
├── src/
│   ├── lib.rs              # public test helpers
│   ├── cli.rs              # CliTest / CliAssert builders
│   ├── tui.rs              # TmuxTuiTest builder
│   ├── tools.rs            # static fake tool registry
│   ├── fixtures.rs         # fixture path resolution
│   └── predicates.rs       # pane/stdout assertion helpers
├── tests/
│   ├── cli_smoke.rs        # Phase 1 CLI tests
│   └── tui_smoke.rs        # Phase 1 TUI tests
├── fixtures/
│   ├── openai/             # OpenAI-compatible .sse fixtures
│   └── anthropic/          # Anthropic-compatible .sse fixtures
└── scripts/
    ├── record_opencode_go.py
    └── record_opencode_go_multiturn.py
```

## Files to modify in runie

| File | Change |
|------|--------|
| `crates/runie-provider/Cargo.toml` | Add `replay` feature flag. |
| `crates/runie-provider/src/lib.rs` | Re-export `ReplayProvider` under the feature flag. |
| `crates/runie-provider/src/factory.rs` | Check `RUNIE_REPLAY_FIXTURES` and return a replay provider. |
| `crates/runie-provider/src/replay.rs` | New module: replay provider that consumes SSE files from disk. |
| `crates/runie-cli/Cargo.toml` | Enable `runie-provider/replay` feature. |
| `crates/runie-tui/Cargo.toml` | Enable `runie-provider/replay` feature. |

No Rust test files are added to the runie repo; all black-box tests live in
`runie-tests`.

## Replay provider behavior

When `RUNIE_REPLAY_FIXTURES` is set, `BuiltProviderFactory` must:

1. Parse the comma-separated list of filesystem paths.
2. Detect protocol from path (`.../openai/...` vs `.../anthropic/...`) or from
   optional `RUNIE_REPLAY_PROTOCOL`.
3. Read each `.sse` file into memory.
4. Construct a `ReplayProvider` that replays fixtures round-robin.
5. Skip API-key/base-URL resolution.

For tool-call fixtures, the replay provider must look up the tool name in a
static registry and emit a matching `ToolResult` event instead of invoking a
real tool. Phase 1 registry:

| Tool name | Output |
|-----------|--------|
| `get_weather` | `{"temperature":22,"unit":"celsius","condition":"sunny"}` |
| `read_file` | `{"path":"README.md","content":"# Example Project\n\nThis is a sample README.\n"}` |
| `list_dir` | `{"entries":["README.md","Cargo.toml"]}` |

## Fixture migration

The fixtures currently committed to
`crates/runie-testing/src/fixtures/{openai,anthropic}/` in the runie repo must
be moved to `runie-tests/fixtures/{openai,anthropic}/`. The recording scripts
currently in `runie/scripts/` must be moved to `runie-tests/scripts/`.

After migration, the runie repo keeps only the replay provider implementation;
the fixtures and black-box harness live in runie-tests.

## DSL shape in runie-tests

The black-box DSL is implemented inside the `runie-tests` crate. See
`tasks/black-box-replay-dsl.md` for the proposed builder API.

Example Phase 1 CLI test:

```rust
use runie_tests::prelude::*;

#[test]
fn print_replays_simple_text() {
    test_cli()
        .fixture("openai/opencode_go_deepseek_v4_flash_simple.sse")
        .args(["print", "say ok"])
        .assert()
        .stdout(contains("ok"))
        .success();
}
```

Example Phase 1 multi-turn CLI test:

```rust
#[test]
fn weather_chain_replays() {
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
        .stdout(contains("Paris"))
        .success();
}
```

Example Phase 1 TUI test:

```rust
#[test]
fn tui_displays_streaming_text() {
    test_tui()
        .fixture("openai/opencode_go_kimi_k2_6_simple.sse")
        .type_keys("say ok")
        .submit()
        .wait_for_idle(Duration::from_millis(500))
        .capture_pane()
        .assert(contains("ok"));
}
```

## Phase 1 smoke matrix

| Scenario | Protocol | Fixture example | Target |
|---|---|---|---|
| simple text | openai | `opencode_go_deepseek_v4_flash_simple.sse` | CLI print |
| simple text | anthropic | `opencode_go_minimax_m3_simple.sse` | CLI print |
| tool call | openai | `opencode_go_deepseek_v4_flash_tool.sse` | CLI json |
| tool call | anthropic | `opencode_go_minimax_m3_tool.sse` | CLI json |
| reasoning | openai | `opencode_go_deepseek_v4_flash_reasoning.sse` | CLI print |
| multi-tool | anthropic | `opencode_go_minimax_m3_multi_tool.sse` | CLI json |
| math chain | openai | `opencode_go_deepseek_v4_pro_multiturn_math_chain_turn{1,2}.sse` | CLI print |
| weather chain | anthropic | `opencode_go_minimax_m3_multiturn_weather_chain_turn{1,2}.sse` | CLI print |
| simple chat | openai | `opencode_go_kimi_k2_6_simple.sse` | TUI tmux |
| tool chat | anthropic | `opencode_go_minimax_m3_tool.sse` | TUI tmux |
| reasoning chat | openai | `opencode_go_deepseek_v4_flash_reasoning.sse` | TUI tmux |
| multi-turn chat | openai | `opencode_go_kimi_k2_6_multiturn_math_chain_turn{1,2}.sse` | TUI tmux |

## Implementation steps

1. **Create `runie-tests/` crate** with `Cargo.toml`, no dependency on runie crates.
2. **Move fixtures and scripts** from `runie/crates/runie-testing/src/fixtures/` and
   `runie/scripts/` to `runie-tests/fixtures/` and `runie-tests/scripts/`.
3. **Implement replay provider** in runie (env-var fixture loading + static tool
   registry + feature flags).
4. **Implement CLI builder** in `runie-tests/src/cli.rs`.
5. **Implement tmux harness** in `runie-tests/src/tui.rs`.
6. **Write Phase 1 smoke tests** in `runie-tests/tests/cli_smoke.rs` and
   `runie-tests/tests/tui_smoke.rs`.
7. **Run the suite locally** and verify deterministic passes.

## Acceptance criteria for Phase 1

- [ ] `runie-tests` crate builds independently.
- [ ] `cargo test` in `runie-tests/` builds runie from `../runie` and runs all
      smoke tests.
- [ ] CLI print/json tests pass for all fixtures in the Phase 1 matrix.
- [ ] TUI tmux tests pass for all fixtures in the Phase 1 matrix.
- [ ] No API keys or network calls are required.
- [ ] Tests are deterministic on repeated runs.
- [ ] No CI is added.

## Open questions / follow-ups

- Should the `runie-tests` crate also include a YAML/TOML runner on top of the
  Rust DSL for easier bulk test additions?
- Should error fixtures (e.g. `rate_limit_error.sse`) be included in Phase 1
  CLI tests or deferred to Phase 2?
- How should the tmux harness handle terminal color/formatting? Strip ANSI
  before assertions?
- Should the runie repo keep a small subset of fixtures for its own provider
  unit tests, or should all fixtures move to `runie-tests`?
