# DSL for black-box replay tests in runie-tests

## Goal

Design an ergonomic Rust DSL for writing black-box tests in the standalone
`runie-tests` crate. The DSL has zero code dependency on runie — it only
spawns runie binaries as subprocesses and asserts on their observable output.

## Constraints

- Test suite lives in `~/code/GitHub/runie-tests/`.
- No `use runie_*` imports.
- Binaries are built from `../runie` source during test setup.
- Fixtures are read from `runie-tests/fixtures/{openai,anthropic}/`.
- Tests run real `tmux` sessions for TUI coverage.

## Principles

- **Fixture-first**: tests start from a fixture path or a list of paths.
- **Multi-turn is a sequence**: a test is a list of fixtures; single-turn tests
  are sequences of length one.
- **CLI and TUI share patterns**: both use builders, both use predicate
  assertions, both support multi-turn.
- **No secrets, no network**: the DSL always sets replay mode.

## Entry points

```rust
use runie_tests::prelude::*;

// CLI test
#[test]
fn cli_example() {
    test_cli()
        .fixture("openai/opencode_go_deepseek_v4_flash_simple.sse")
        .args(["print", "say ok"])
        .assert()
        .stdout(contains("ok"))
        .success();
}

// TUI test
#[test]
fn tui_example() {
    test_tui()
        .fixture("openai/opencode_go_kimi_k2_6_simple.sse")
        .type_keys("say ok")
        .submit()
        .wait_for_idle(Duration::from_millis(500))
        .capture_pane()
        .assert(contains("ok"));
}
```

## Core types

```rust
/// Start a CLI black-box test.
pub fn test_cli() -> CliTest;

/// Start a TUI black-box test inside a tmux session.
pub fn test_tui() -> TuiTest;

/// Helpers imported via `runie_tests::prelude::*`.
pub mod predicates {
    pub fn contains<S: AsRef<str>>(text: S) -> impl Predicate<str>;
    pub fn not_contains<S: AsRef<str>>(text: S) -> impl Predicate<str>;
}
```

## CLI builder

```rust
pub struct CliTest {
    fixtures: Vec<PathBuf>,
    args: Vec<String>,
    env: Vec<(String, String)>,
    home: Option<TempDir>,
}

impl CliTest {
    /// Add one fixture.
    pub fn fixture<P: AsRef<Path>>(self, path: P) -> Self;

    /// Add multiple fixtures (multi-turn).
    pub fn fixtures<I, P>(self, paths: I) -> Self;

    /// CLI arguments after `runie`.
    pub fn args<I, S>(self, args: I) -> Self;

    /// Extra env var.
    pub fn env<K, V>(self, key: K, value: V) -> Self;

    /// Build and run.
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

### CLI examples

Single-turn:

```rust
#[test]
fn print_simple_text() {
    test_cli()
        .fixture("openai/opencode_go_deepseek_v4_flash_simple.sse")
        .args(["print", "say ok"])
        .assert()
        .stdout(contains("ok"))
        .success();
}
```

Multi-turn:

```rust
#[test]
fn weather_chain() {
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
        .stdout(contains("Paris").and(contains("Berlin")))
        .success();
}
```

Tool-call fixture:

```rust
#[test]
fn json_tool_call() {
    test_cli()
        .fixture("openai/opencode_go_deepseek_v4_flash_tool.sse")
        .args(["json", "--model", "opencode-go/deepseek-v4-flash", "weather in Paris"])
        .assert()
        .stdout(contains("get_weather"))
        .success();
}
```

Anthropic protocol:

```rust
#[test]
fn anthropic_simple() {
    test_cli()
        .fixture("anthropic/opencode_go_minimax_m3_simple.sse")
        .env("RUNIE_REPLAY_PROTOCOL", "anthropic")
        .args(["print", "say ok"])
        .assert()
        .stdout(contains("ok"))
        .success();
}
```

## TUI builder

```rust
pub struct TuiTest {
    fixtures: Vec<PathBuf>,
    keys: Vec<KeyEvent>,
    idle_timeout: Duration,
    home: Option<TempDir>,
}

impl TuiTest {
    pub fn fixture<P: AsRef<Path>>(self, path: P) -> Self;
    pub fn fixtures<I, P>(self, paths: I) -> Self;

    /// Type literal text.
    pub fn type_keys<S: AsRef<str>>(self, text: S) -> Self;

    /// Press Enter.
    pub fn submit(self) -> Self;

    /// Press a specific key.
    pub fn press(self, key: KeyEvent) -> Self;

    /// Wait for pane content to stop changing.
    pub fn wait_for_idle(self, timeout: Duration) -> Self;

    /// Capture tmux pane content.
    pub fn capture_pane(self) -> TuiAssert;
}

pub struct TuiAssert {
    pane_text: String,
}

impl TuiAssert {
    pub fn assert(self, pred: impl Predicate<str>);
    pub fn stdout(self, pred: impl Predicate<str>) -> Self; // tmux pane == stdout
    pub fn stderr(self, pred: impl Predicate<str>) -> Self; // captured separately
}
```

### TUI examples

Basic chat:

```rust
#[test]
fn tui_simple_chat() {
    test_tui()
        .fixture("openai/opencode_go_kimi_k2_6_simple.sse")
        .type_keys("say ok")
        .submit()
        .wait_for_idle(Duration::from_millis(500))
        .capture_pane()
        .assert(contains("ok"));
}
```

Tool-call rendering:

```rust
#[test]
fn tui_shows_tool_call() {
    test_tui()
        .fixture("openai/opencode_go_deepseek_v4_flash_tool.sse")
        .type_keys("weather in Paris")
        .submit()
        .wait_for_idle(Duration::from_millis(500))
        .capture_pane()
        .assert(contains("get_weather"));
}
```

Multi-turn chat:

```rust
#[test]
fn tui_multi_turn() {
    test_tui()
        .fixtures([
            "openai/opencode_go_kimi_k2_6_multiturn_math_chain_turn1.sse",
            "openai/opencode_go_kimi_k2_6_multiturn_math_chain_turn2.sse",
        ])
        .type_keys("What is 2 + 2?")
        .submit()
        .wait_for_idle(Duration::from_millis(500))
        .type_keys("Multiply that by 3.")
        .submit()
        .wait_for_idle(Duration::from_millis(500))
        .capture_pane()
        .assert(contains("6"));
}
```

## Fixture resolution

The DSL resolves fixture paths relative to a root directory that defaults to
`runie-tests/fixtures/`:

```rust
pub fn fixture_path(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("fixtures")
        .join(name)
}
```

For multi-turn tests, the list order maps to user turns in order. The replay
provider cycles through fixtures round-robin.

## Environment setup

Every test gets a temp `HOME` directory containing `~/.runie/config.toml`:

```toml
provider = "opencode-go"
model = "deepseek-v4-flash"
```

The DSL sets:

```text
HOME=<temp_dir>
RUNIE_REPLAY_FIXTURES=<fixture_paths>
RUNIE_REPLAY_PROTOCOL=<openai|anthropic> (optional)
```

and unsets any real provider API-key env vars to prevent accidental live calls.

## Binary discovery

The DSL locates the compiled binaries using cargo's automatic env vars once
`runie-tests/Cargo.toml` declares the binaries as dependencies:

```toml
[[bin]]
name = "runie"
path = "../runie/crates/runie-cli/src/main.rs"

[[bin]]
name = "runie-tui"
path = "../runie/crates/runie-tui/src/main.rs"
```

Alternatively, a build script in `runie-tests` runs `cargo build` in `../runie`
and records `target/debug/runie` and `target/debug/runie-tui` paths.

Recommended approach for Phase 1: a `build.rs` in `runie-tests` that builds
`../runie` and sets env vars pointing to the resulting binaries.

## Tool mocking

The replay provider inside runie must supply fake tool outputs. The DSL does
not configure this per-test in Phase 1; it relies on a static registry keyed
by tool name.

If per-test custom tool output is needed later, the DSL can set:

```text
RUNIE_REPLAY_TOOL_GET_WEATHER={"temperature":-5,"unit":"celsius"}
```

and expose:

```rust
test_cli()
    .fixture("...")
    .tool("get_weather", json!({"temperature": -5}))
    .args([...])
    .assert();
```

## Predicate helpers

```rust
pub fn contains<S: AsRef<str>>(text: S) -> impl Predicate<str>;
pub fn not_contains<S: AsRef<str>>(text: S) -> impl Predicate<str>;
pub fn matches<S: AsRef<str>>(regex: S) -> impl Predicate<str>;

// Convenience for chaining
pub fn and<A, B>(a: A, b: B) -> impl Predicate<str>;
pub fn or<A, B>(a: A, b: B) -> impl Predicate<str>;
```

These wrap a small internal predicate trait so the DSL does not depend on
`predicates`/`assert_cmd` crates if we want to keep dependencies minimal.

## Error reporting

On failure, print:

- Test name and fixture paths
- CLI args or TUI key sequence
- Actual stdout / pane text
- Expected predicate
- Exit code
- tmux session/pane ID if applicable

## Suggested file layout in runie-tests

```text
runie-tests/src/
├── lib.rs          # prelude re-exports
├── cli.rs          # CliTest, CliAssert
├── tui.rs          # TuiTest, TuiAssert, tmux helpers
├── fixtures.rs     # fixture path resolution
├── predicates.rs   # predicate trait and helpers
├── tools.rs        # static tool registry (optional Phase 2)
└── env.rs          # temp HOME and env isolation
```

## Relationship to runie

- No imports from `runie_*` crates.
- No shared code with runie.
- Only contract is the subprocess CLI and the `RUNIE_REPLAY_*` env vars.

## Acceptance criteria

- [ ] A Rust developer can write a single-turn CLI test in <8 lines.
- [ ] A multi-turn CLI test requires only adding fixtures to `.fixtures([...])`.
- [ ] A TUI tmux test can assert on captured pane text.
- [ ] The DSL builds without any runie crate dependency.
- [ ] Tests are deterministic and require no API keys.
