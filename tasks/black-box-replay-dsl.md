# DSL for black-box replay tests in runie-tests

## Goal

Design a small, ergonomic DSL that makes it trivial to write black-box tests
against the recorded OpenCode Go fixtures. The DSL should handle single-turn
and multi-turn conversations, CLI and TUI targets, and common assertion
patterns without repeating boilerplate.

No code changes in this task — design only.

## Principles

- **Fixture-first**: tests start from a named fixture or a list of fixtures,
  not from provider construction.
- **Multi-turn is the default shape**: a test is a sequence of turns, where a
  single-turn test is just a sequence of length one.
- **CLI and TUI share the same surface**: the DSL abstracts over spawning a
  subprocess vs driving an in-process `TuiRuntime`.
- **Readable assertions**: stdout/stderr/exit-code assertions read like
  predicates; render assertions are opt-in for TUI.
- **No secrets, no network**: DSL-enforced defaults that only use replay mode.

## Proposed Rust builder DSL

Add a `runie-testing` helper module (e.g. `black_box`) that exposes a fluent
builder. This is the recommended layer-5 test style.

### Single-turn CLI test

```rust
use runie_testing::black_box::cli;

#[test]
fn simple_text_replays() {
    cli()
        .fixtures(["openai/opencode_go_deepseek_v4_flash_simple.sse"])
        .args(["print", "say ok"])
        .assert()
        .stdout(contains("ok"))
        .success();
}
```

### Multi-turn CLI test

```rust
#[test]
fn weather_chain_replays() {
    cli()
        .fixtures([
            "openai/opencode_go_deepseek_v4_pro_multiturn_weather_chain_turn1.sse",
            "openai/opencode_go_deepseek_v4_pro_multiturn_weather_chain_turn2.sse",
        ])
        .args(["print", "What is the weather in Paris?", "What about Berlin?"])
        .assert()
        .stdout(contains("Paris").and(contains("Berlin")))
        .success();
}
```

### Tool-call fixture with canned tool output

```rust
#[test]
fn tool_call_replays() {
    cli()
        .fixtures(["openai/opencode_go_deepseek_v4_flash_tool.sse"])
        .tools([get_weather().returns(json!({"temperature": 22, "unit": "celsius"}))])
        .args(["json", "--model", "opencode-go/deepseek-v4-flash", "weather in Paris"])
        .assert()
        .stdout(contains("get_weather"))
        .success();
}
```

### TUI test with TestBackend

```rust
use runie_testing::black_box::tui;

#[test]
fn tui_displays_streaming_text() {
    tui()
        .fixtures(["openai/opencode_go_kimi_k2_6_simple.sse"])
        .type_keys("say ok")
        .submit()
        .wait_for(rendered(contains("ok")))
        .assert();
}
```

### TUI test with raw terminal (tmux / expect)

```rust
#[test]
fn tui_runs_in_terminal() {
    tui_terminal()
        .fixtures(["openai/opencode_go_kimi_k2_6_simple.sse"])
        .type_keys("say ok")
        .submit()
        .snapshot("tui_simple_ok")
        .assert();
}
```

## Key DSL types

```rust
/// Entry point for CLI black-box tests.
pub fn cli() -> CliTest;

/// Entry point for TUI black-box tests (TestBackend).
pub fn tui() -> TuiTest;

/// Entry point for TUI terminal tests (tmux/expect).
pub fn tui_terminal() -> TerminalTuiTest;

pub struct CliTest {
    fixtures: Vec<String>,
    args: Vec<String>,
    tools: Vec<MockTool>,
    env: Vec<(String, String)>,
    config_overrides: Vec<(String, toml::Value)>,
}

impl CliTest {
    pub fn fixtures<I, S>(self, fixtures: I) -> Self;
    pub fn args<I, S>(self, args: I) -> Self;
    pub fn tools<I>(self, tools: I) -> Self;
    pub fn env<K, V>(self, key: K, value: V) -> Self;
    pub fn assert(self) -> CliAssert;
}

pub struct CliAssert {
    output: std::process::Output,
}

impl CliAssert {
    pub fn stdout(self, pred: impl Predicate<str>) -> Self;
    pub fn stderr(self, pred: impl Predicate<str>) -> Self;
    pub fn exit_code(self, code: i32) -> Self;
    pub fn success(self) -> Self;
    pub fn failure(self) -> Self;
}
```

## Declarative YAML spec (optional bulk runner)

For quickly adding many similar tests without writing Rust, support a YAML
spec interpreted by a single Rust test runner.

```yaml
# crates/runie-cli/tests/replay_blackbox.yaml
fixtures_root: "../../runie-testing/src/fixtures"

tests:
  - name: simple_text
    target: cli
    fixtures:
      - openai/opencode_go_deepseek_v4_flash_simple.sse
    args: ["print", "say ok"]
    assert:
      stdout_contains: "ok"
      exit_code: 0

  - name: weather_chain
    target: cli
    fixtures:
      - openai/opencode_go_deepseek_v4_pro_multiturn_weather_chain_turn1.sse
      - openai/opencode_go_deepseek_v4_pro_multiturn_weather_chain_turn2.sse
    args: ["print", "What is the weather in Paris?", "What about Berlin?"]
    assert:
      stdout_contains:
        - "Paris"
        - "Berlin"
      exit_code: 0

  - name: minimax_simple_anthropic
    target: cli
    protocol: anthropic
    fixtures:
      - anthropic/opencode_go_minimax_m3_simple.sse
    args: ["print", "say ok"]
    assert:
      stdout_contains: "ok"
```

A single Rust test loads the YAML and runs every entry:

```rust
#[test]
fn replay_yaml_suite() {
    runie_testing::black_box::run_yaml_suite("tests/replay_blackbox.yaml");
}
```

This is optional. The builder DSL is the primary target.

## Multi-turn semantics

- `.fixtures([a, b, c])` maps to `RUNIE_REPLAY_FIXTURES=a,b,c`.
- The replay provider consumes fixtures round-robin: turn 1 uses fixture 0,
  turn 2 uses fixture 1, etc.
- If there are more turns than fixtures, the provider wraps around. Tests
  should normally provide exactly one fixture per user turn.
- For CLI tests, each fixture corresponds to one `print`/`json` invocation or
  one prompt in a multi-prompt invocation.

## Tool mocking in the DSL

The DSL should build a static tool registry that the replay provider reads
from an env var or a temp file:

```rust
fn get_weather() -> MockTool;
fn read_file() -> MockTool;
fn list_dir() -> MockTool;

MockTool::new("get_weather")
    .when(json!({"city": "Paris"}))
    .returns(json!({"temperature": 22, "unit": "celsius"}))
```

At runtime, the DSL serializes the registry and sets:

```text
RUNIE_REPLAY_TOOLS={"get_weather":{"default":{"temperature":22,...}},...}
```

The replay provider uses this registry to answer `ToolCallStart` events
instead of invoking real tools.

## TUI DSL design

For `tui()` (TestBackend):

```rust
tui()
    .fixtures([...])
    .type_keys("say ok")          // type literal text
    .press(KeyCode::Enter)        // press a key
    .submit()                     // convenience for Enter
    .wait_for(event(|e| matches!(e, Event::Done { .. })))
    .assert_buffer(|buf| {
        assert!(buf_contains(buf, "ok"));
    });
```

For `tui_terminal()` (tmux/expect):

```rust
tui_terminal()
    .fixtures([...])
    .type_keys("say ok")
    .submit()
    .wait_for_screen(contains("ok"))
    .assert();
```

## Configuration overrides

Some black-box tests need an isolated home directory or specific config:

```rust
cli()
    .fixtures([...])
    .config("provider", "opencode-go")
    .config("model", "deepseek-v4-flash")
    .args(["print", "hi"])
    .assert()
    .success();
```

The DSL creates a temp home, writes `~/.runie/config.toml`, and sets `HOME`
for the subprocess.

## Error reporting

- On failure, print the fixture names, the CLI args, and the actual stdout/stderr.
- For TUI failures, print the final `Buffer` diff or terminal screen capture.
- Include which turn failed in multi-turn tests.

## Suggested file layout

```text
crates/runie-testing/src/black_box/
├── mod.rs          # public entry points: cli(), tui(), tui_terminal()
├── cli.rs          # CliTest / CliAssert
├── tui.rs          # TuiTest for TestBackend
├── terminal.rs     # TerminalTuiTest for tmux/expect
├── tools.rs        # MockTool builders
└── yaml.rs         # optional YAML spec runner
```

## Relationship to existing runie-testing modules

- Reuse `fixtures::openai::fixture` and `fixtures::anthropic::fixture` to
  resolve fixture paths.
- Reuse `with_env`/`EnvGuard` for environment isolation.
- Reuse `keystroke_dsl` for TUI key sequences.
- Reuse `capture_events` for event-based assertions.

## Acceptance criteria for the DSL task

- [ ] A Rust developer can write a single-turn CLI black-box test in <5 lines.
- [ ] Multi-turn CLI tests require only adding more fixtures to `.fixtures([...])`.
- [ ] Tool-call fixtures can be paired with deterministic tool responses.
- [ ] TUI tests can assert on rendered output without spawning a real terminal.
- [ ] Optional YAML runner lets non-Rust contributors add fixture-driven tests.
- [ ] DSL implementation lives in `runie-testing` and is gated behind a
      `black-box` feature.

## Open questions

- Should the DSL support snapshots (insta) for TUI buffer assertions?
- Should CLI tests use `assert_cmd`/`predicates` directly, or wrap them to
  enforce replay-only mode?
- Should fixture paths be relative to `crates/runie-testing/src/fixtures/` or
  absolute from workspace root?
