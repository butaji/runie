# Black-box replay testing for runie CLI and TUI

## Goal

Make the real `runie-cli` and `runie-tui` binaries runnable against recorded
SSE fixtures so we can write black-box tests that do not depend on live model
APIs. This builds on the OpenCode Go fixtures already recorded in
`crates/runie-testing/src/fixtures/`.

## Background

- `runie-cli` and `runie-tui` both construct providers through
  `BuiltProviderFactory` in `crates/runie-provider/src/factory.rs`.
- `runie-testing` already has a `ReplayProvider` that cycles through SSE
  fixtures and emits `ProviderEvent`s. It is currently used only by Rust
  unit tests that construct the provider directly.
- There is no way for the compiled binaries to use fixtures; the only
  non-production mode is `RUNIE_MOCK=1`, which uses hardcoded `MockProvider`
  responses.

## Strategy

Add an environment-variable replay mode to `BuiltProviderFactory`. When
`RUNIE_REPLAY_FIXTURES` is set, the factory loads the listed SSE files from
disk, constructs a `ReplayProvider`, and returns it without resolving API keys
or base URLs. This keeps the change inside `runie-provider` and requires no
modifications to `runie-cli` or `runie-tui` entry points.

## Files to modify

| File | Change |
|------|--------|
| `crates/runie-provider/Cargo.toml` | Add `replay` feature flag. |
| `crates/runie-provider/src/lib.rs` | Re-export `ReplayProvider` under the feature flag. |
| `crates/runie-provider/src/factory.rs` | Check `RUNIE_REPLAY_FIXTURES` and return a replay provider. |
| `crates/runie-provider/src/replay.rs` | New module: move/adapt `ReplayProvider` from `runie-testing`. |
| `crates/runie-cli/Cargo.toml` | Enable `runie-provider/replay` feature. |
| `crates/runie-tui/Cargo.toml` | Enable `runie-provider/replay` feature (extend existing `mock` feature setup). |

## Implementation steps

### 1. Add the `replay` feature

In `crates/runie-provider/Cargo.toml`:

```toml
[features]
default = ["openai"]
openai = []
mock = []
replay = []
```

### 2. Create `crates/runie-provider/src/replay.rs`

Move/adapt the provider implementation from
`crates/runie-testing/src/replay_provider.rs`. The replay provider must:

- Accept a list of raw SSE strings and a protocol (`openai` or `anthropic`).
- Implement the `Provider` trait by returning a stream of `ProviderEvent`s.
- Cycle through fixtures round-robin (to support multi-turn tests).
- Support both protocols:
  - `openai` → use `runie_provider::openai::stream::replay_sse`.
  - `anthropic` → use `runie_provider::anthropic::replay_anthropic_sse`.

Keep the public API minimal:

```rust
pub struct ReplayProvider {
    fixtures: Vec<String>,
    protocol: Protocol,
    cursor: AtomicUsize,
}

pub enum Protocol { OpenAi, Anthropic }

impl ReplayProvider {
    pub fn new(fixtures: Vec<String>, protocol: Protocol) -> Self;
}

impl Provider for ReplayProvider {
    fn generate(&self, _messages: Vec<ChatMessage>) -> ProviderStream {
        let fixture = self.fixtures[self.cursor.fetch_add(1, Ordering::Relaxed) % self.fixtures.len()].clone();
        let events = match self.protocol {
            Protocol::OpenAi => runie_provider::openai::stream::replay_sse(&fixture),
            Protocol::Anthropic => runie_provider::anthropic::replay_anthropic_sse(&fixture),
        };
        Box::pin(futures::stream::iter(events.into_iter().map(Ok)))
    }
}
```

### 3. Wire `ReplayProvider` into `BuiltProviderFactory`

In `crates/runie-provider/src/factory.rs`, before the normal provider build
path:

```rust
#[cfg(feature = "replay")]
if let Ok(fixture_list) = std::env::var("RUNIE_REPLAY_FIXTURES") {
    let paths: Vec<&str> = fixture_list.split(',').map(str::trim).collect();
    let mut fixtures = Vec::new();
    let mut protocol = Protocol::OpenAi;

    for path in paths {
        if path.is_empty() {
            continue;
        }
        // Infer protocol from path or env var.
        if path.contains("/anthropic/") {
            protocol = Protocol::Anthropic;
        }
        let contents = std::fs::read_to_string(path)
            .map_err(|e| anyhow::anyhow!("failed to read replay fixture {path}: {e}"))?;
        fixtures.push(contents);
    }

    if !fixtures.is_empty() {
        let replay = ReplayProvider::new(fixtures, protocol);
        return Ok(BuiltProvider::from_provider(
            Box::new(replay),
            provider,
            model,
        ));
    }
}
```

Also support `RUNIE_REPLAY_PROTOCOL=anthropic` for explicit protocol selection.

### 4. Export `ReplayProvider` under the feature flag

In `crates/runie-provider/src/lib.rs`:

```rust
#[cfg(feature = "replay")]
pub mod replay;

#[cfg(feature = "replay")]
pub use replay::{ReplayProvider, Protocol as ReplayProtocol};
```

### 5. Enable the feature in binaries

In `crates/runie-cli/Cargo.toml`:

```toml
[dependencies]
runie-provider = { workspace = true, features = ["replay"] }
```

In `crates/runie-tui/Cargo.toml`, extend the existing runie-provider dep:

```toml
runie-provider = { workspace = true, features = ["mock", "replay"] }
```

### 6. Mock tool outputs for black-box tests

Tool-call fixtures need deterministic tool responses. Provide two mechanisms:

1. **Static tool registry** inside `ReplayProvider`: for known tool names
   (`get_weather`, `list_dir`, `read_file`, `bash`), return a small canned
   JSON payload. This covers the existing fixtures without extra config.
2. **Env var override** for custom responses:
   `RUNIE_REPLAY_TOOL_<NAME>='{"result":"..."}'`.

For the initial implementation, the static registry is enough. Wire it through
`runie_agent` by keeping the existing `MockToolSkill` path available in tests,
or by making the replay provider emit `ToolCallStart`/`ToolCallEnd` and letting
the agent invoke real built-in tools for read-only operations.

### 7. Add black-box tests

#### CLI black-box tests

Create `crates/runie-cli/tests/replay_blackbox.rs` using `assert_cmd` and
`predicates` (add to `dev-dependencies` if absent):

```rust
use assert_cmd::Command;
use std::path::PathBuf;

fn fixture(name: &str) -> PathBuf {
    PathBuf::from("../../runie-testing/src/fixtures/openai").join(name)
}

#[test]
fn print_replays_simple_text() {
    Command::cargo_bin("runie")
        .unwrap()
        .arg("print")
        .arg("say ok")
        .env("RUNIE_REPLAY_FIXTURES", fixture("opencode_go_deepseek_v4_flash_simple.sse"))
        .assert()
        .success()
        .stdout(predicates::str::contains("ok").or(predicates::str::contains("ok")));
}

#[test]
fn json_replays_tool_call() {
    Command::cargo_bin("runie")
        .unwrap()
        .arg("json")
        .arg("--model")
        .arg("opencode-go/deepseek-v4-flash")
        .arg("weather in Paris")
        .env("RUNIE_REPLAY_FIXTURES", fixture("opencode_go_deepseek_v4_flash_tool.sse"))
        .assert()
        .success()
        .stdout(predicates::str::contains("get_weather"));
}
```

#### TUI black-box tests

Two tiers:

1. **Deterministic actor-level tests** (recommended for CI): keep using
   `TuiRuntime` with `TestBackend`, but drive it with a config that selects a
   replay provider via env var. This is already close to the existing
   `crates/runie-tui/src/tests/` style.
2. **True binary tests** (manual / nightly): spawn the compiled `runie-tui`
   inside `tmux` or `expect`, feed keystrokes, and assert on screen capture.
   These catch terminal interaction bugs but are slower.

For tier 1, add a test in `crates/runie-tui/src/tests/`:

```rust
#[test]
fn replay_provider_drives_turn() {
    with_env(|env| {
        env.set("RUNIE_REPLAY_FIXTURES", fixture_path("opencode_go_kimi_k2_6_simple.sse"));
        let (tx, mut rx) = event_channel();
        let runtime = TuiRuntime::builder()
            .backend(BackendType::Test)
            .event_sink(tx)
            .build();
        // Drive input, assert on events or final buffer.
    });
}
```

### 8. Update documentation

- `docs/BlackBoxTesting.md` — user-facing guide for running binaries in replay
  mode.
- `crates/runie-testing/src/fixtures/opencode_go/README.md` — mention how to
  use OpenCode Go fixtures in black-box tests.
- `AGENTS.md` — add black-box replay tests as an extension of layer 4.

## Acceptance criteria

- [ ] `RUNIE_REPLAY_FIXTURES=path/to/fixture.sse cargo run -p runie-cli -- print "hello"`
      produces deterministic output without network calls.
- [ ] `RUNIE_REPLAY_FIXTURES=path/to/fixture.sse cargo run -p runie-tui` starts
      the TUI and drives a turn from the fixture.
- [ ] `cargo test -p runie-cli` includes at least one black-box replay test.
- [ ] `cargo test -p runie-tui` includes at least one replay-driven test.
- [ ] Existing workspace tests still pass.
- [ ] No API keys or secrets are required to run the new tests.
- [ ] Docs and `AGENTS.md` describe the new testing layer.

## Multi-step / multi-turn fixtures

The OpenCode Go fixtures now include per-turn recordings of multi-step
conversations in
`crates/runie-testing/src/fixtures/{openai,anthropic}/opencode_go_*_multiturn_*_turn*.sse`.
These are essential for black-box tests that exercise conversation history,
follow-up questions, and tool-result replay.

### Recorded scenarios

| Scenario | Turns | Tool use | Models recorded |
|---|---|---|---|
| `math_chain` | 2 | No | deepseek-v4-pro, deepseek-v4-flash, glm-5.2, kimi-k2.6, minimax-m3, qwen3.7-max |
| `weather_chain` | 2 | Yes | deepseek-v4-pro, deepseek-v4-flash, glm-5.2, kimi-k2.6, minimax-m3, qwen3.7-max |
| `read_summarize_followup` | 2 | Yes | deepseek-v4-pro, deepseek-v4-flash, minimax-m3, qwen3.7-max |
| `reasoning_followup` | 2 | No | deepseek-v4-pro, deepseek-v4-flash, glm-5.2, kimi-k2.6, minimax-m3, qwen3.7-max |
| `multi_tool_then_compare` | 2 | Yes | deepseek-v4-pro, deepseek-v4-flash, minimax-m3, qwen3.7-max |
| `clarification` | 2 | No | deepseek-v4-pro, deepseek-v4-flash, minimax-m3, qwen3.7-max |

Total: **60 per-turn fixtures** (36 OpenAI-compatible, 24 Anthropic-compatible)
across 30 model/scenario pairs.

### Recording harness

`scripts/record_opencode_go_multiturn.py` replays a scripted conversation
against OpenCode Go, injects fake tool results, and records each turn as a
separate sanitized fixture. Re-run with:

```bash
export OPENCODE_GO_API_KEY=sk-...
python3 scripts/record_opencode_go_multiturn.py
```

### How to use in black-box tests

A multi-turn CLI/TUI test feeds the fixtures in order. The replay provider
cycles through `RUNIE_REPLAY_FIXTURES` round-robin, so turn *n* of the test
uses turn *n* of the fixture list:

```bash
RUNIE_REPLAY_FIXTURES="\
crates/runie-testing/src/fixtures/openai/opencode_go_deepseek_v4_pro_multiturn_weather_chain_turn1.sse,\
crates/runie-testing/src/fixtures/openai/opencode_go_deepseek_v4_pro_multiturn_weather_chain_turn2.sse" \
  cargo run -p runie-cli -- print "What is the weather in Paris?" "What about Berlin?"
```

### Implementation notes specific to multi-turn

- The replay provider must cycle fixtures, not stop after the first one.
- Tool results injected during recording are fake and deterministic; the
  replay provider must provide matching fake results when the fixture emits
  `ToolCallStart`.
- Each turn fixture is a complete SSE stream and can be replayed standalone
  for unit tests, or chained for end-to-end conversation tests.
- Update `crates/runie-testing/src/fixtures/openai.rs` and
  `crates/runie-testing/src/fixtures/anthropic.rs` `ALL_*_FIXTURES` lists to
  include the new `_multiturn_*_turn*.sse` names.

## Open questions / follow-ups

- Should replay fixtures be embedded at compile time (for hermetic tests) or
  loaded from disk (for easier iteration)? The env-var path approach is
  recommended for iteration; embedding can be added later for CI stability.
- Should `RUNIE_REPLAY_FIXTURES` support directories/globs? Useful for
  multi-turn conversations where each turn uses the next fixture.
- How should tool-call fixtures that require stateful tools (e.g. file edits)
  be handled? Prefer read-only tool fixtures for black-box tests; stateful
  tool tests belong in agent-level Rust tests with `MockToolSkill`.
- Should we record 3+ turn conversations, or add system-prompt / persona
  scenarios? Extend `scripts/record_opencode_go_multiturn.py` with additional
  `SCENARIOS` entries when needed.
- See `tasks/black-box-replay-dsl.md` for a proposed ergonomic DSL that
  consumes these fixtures in `runie-testing` black-box tests.
