# DSL for black-box replay tests in runie-tests

## What this is

The `runie-tests` crate exposes a fluent Rust DSL for black-box testing of the
real `runie-cli` and `runie-tui` binaries. The DSL has zero Rust crate
dependency on `runie`: it locates or builds the binaries, spawns them as
subprocesses, and asserts on observable output. No API keys or network calls are
required at test runtime.

## Constraints (actual)

- Test suite lives in `runie-tests/`.
- No `use runie_*` imports.
- Binaries are located via `RUNIE_BIN` / `RUNIE_CLI_BIN` or built from the
  pinned `runie/` submodule by `src/app_test.rs::ensure_runie_binaries_built()`.
- Fixtures are read from `runie-tests/fixtures/{openai,anthropic}/` via
  `src/fixtures.rs::fixture_path()`.
- TUI tests run real `tmux` sessions via `src/tui.rs`.

## Entry points and prelude

Import everything through `runie_tests::prelude::*`:

```rust
use runie_tests::prelude::*;
```

Public entry points defined in `src/lib.rs`:

```rust
pub fn test_cli() -> CliTest;
pub fn test_tui() -> TuiTest;
```

Predicate helpers imported via prelude:

```rust
pub fn contains<S: Into<String>>(text: S) -> Contains;
pub fn not_contains<S: Into<String>>(text: S) -> NotContains;
pub fn matches<S: AsRef<str>>(pattern: S) -> Matches;
pub fn and<A, B>(a: A, b: B) -> And<A, B> where A: Predicate<str>, B: Predicate<str>;
pub fn or<A, B>(a: A, b: B) -> Or<A, B> where A: Predicate<str>, B: Predicate<str>;
```

## CLI builder (`src/cli.rs`)

```rust
pub struct CliTest {
    fixtures: Vec<PathBuf>,
    args: Vec<String>,
    env_vars: Vec<(String, String)>,
    protocol: Option<String>,
    stdin: Option<Vec<u8>>,
}

impl CliTest {
    pub fn new() -> Self;
    pub fn fixture<P: AsRef<Path>>(self, path: P) -> Self;
    pub fn fixtures<I, P>(self, paths: I) -> Self where I: IntoIterator<Item = P>, P: AsRef<Path>;
    pub fn args<I, S>(self, args: I) -> Self where I: IntoIterator<Item = S>, S: Into<String>;
    pub fn env<K, V>(self, key: K, value: V) -> Self where K: Into<String>, V: Into<String>;
    pub fn with_stdin<B: Into<Vec<u8>>>(self, bytes: B) -> Self;
    pub fn protocol(self, protocol: &str) -> Self; // sets RUNIE_REPLAY_PROTOCOL
    pub async fn assert(self) -> Result<CliAssert>;
    fn create_temp_home(&self) -> Result<TempDir>;
}
```

`assert()` returns `CliAssert` with these methods:

```rust
impl CliAssert {
    pub fn stdout<P>(self, pred: P) -> Self where P: Predicate<str> + 'static;
    pub fn stderr<P>(self, pred: P) -> Self where P: Predicate<str> + 'static;
    pub fn exit_code<P>(self, pred: P) -> Self where P: Predicate<u32> + 'static;
    pub fn success(self) -> Self; // exit code 0
    pub fn failure(self) -> Self; // non-zero exit code
    pub fn stdout_str(&self) -> String;
    pub fn stderr_str(&self) -> String;
    pub fn get_exit_code(&self) -> Option<i32>;
}
```

### CLI examples

Single-turn:

```rust
#[tokio::test]
async fn print_simple_text() {
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

Multi-turn:

```rust
#[tokio::test]
async fn weather_chain() {
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
        .stdout(contains("Berlin"))
        .success();
}
```

Anthropic protocol:

```rust
#[tokio::test]
async fn anthropic_simple() {
    test_cli()
        .fixture("anthropic/opencode_go_minimax_m3_simple.sse")
        .protocol("anthropic")
        .args(["print", "say ok"])
        .assert()
        .await
        .unwrap()
        .stdout(contains("ok"))
        .success();
}
```

## TUI builder (`src/tui.rs`)

```rust
pub struct TuiTest {
    fixtures: Vec<PathBuf>,
    keys: Vec<String>,
    idle_timeout: Duration,
    protocol: Option<String>,
    timeout: TimeoutConfig,
}

impl TuiTest {
    pub fn new() -> Self;
    pub fn fixture<P: AsRef<Path>>(self, path: P) -> Self;
    pub fn fixtures<I, P>(self, paths: I) -> Self where I: IntoIterator<Item = P>, P: AsRef<Path>;
    pub fn type_keys<S: AsRef<str>>(self, text: S) -> Self;
    pub fn submit(self) -> Self; // presses Enter
    pub fn press<K: Into<String>>(self, key: K) -> Self;
    pub fn wait_for_idle(self, timeout: Duration) -> Self;
    pub fn with_timeout(self, timeout: TimeoutConfig) -> Self;
    pub fn protocol(self, protocol: &str) -> Self;
    pub async fn capture_pane(self) -> Result<TuiAssert>;
}
```

`capture_pane()` returns `TuiAssert`:

```rust
impl TuiAssert {
    pub fn assert<P>(self, pred: P) -> Self where P: Predicate<str> + 'static;
    pub fn pane_text(&self) -> &str;
}
```

### TUI examples

Basic chat:

```rust
#[tokio::test]
async fn tui_simple_chat() {
    test_tui()
        .fixture("openai/opencode_go_kimi_k2_6_simple.sse")
        .type_keys("say ok")
        .submit()
        .wait_for_idle(Duration::from_millis(500))
        .capture_pane()
        .await
        .unwrap()
        .assert(contains("ok"));
}
```

Multi-turn chat:

```rust
#[tokio::test]
async fn tui_multi_turn() {
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
        .await
        .unwrap()
        .assert(contains("6"));
}
```

## AppTest TUI harness (`src/app_test.rs`)

For tests that need interactive TUI control (dialogs, pickers, permission
flows), use `AppTest`:

```rust
use runie_tests::{keys, AppTest};

#[tokio::test]
async fn interactive_test() -> Result<()> {
    AppTest::mock()
        .start().await?
        .open_command_palette().await?
        .type_text("switch").await?
        .press(keys::ENTER).await?
        .expect_text("mock/echo").await?;
    Ok(())
}
```

Key constructors:

```rust
AppTest::mock()              // RUNIE_MOCK=1, mock/echo connected
AppTest::mock_cli_flag()     // --mock flag only
AppTest::mock_with_fixture(fixture) // --mock --mock-model <fixture>
AppTest::mock_onboarding()   // --mock-onboarding
AppTest::onboarding()        // no config, forces provider picker
AppTest::configured()        // valid Anthropic config
AppTest::with_config(config) // custom TestConfig
AppTest::normal()            // no config, explicit empty-state
```

Key methods:

```rust
pub async fn start(&mut self) -> Result<&mut Self>;
pub async fn reset_chat(&mut self) -> Result<&mut Self>;
pub async fn press(&mut self, key: impl Into<Key>) -> Result<&mut Self>;
pub async fn type_text(&mut self, text: &str) -> Result<&mut Self>;
pub async fn open_command_palette(&mut self) -> Result<&mut Self>;
pub async fn open_model_switcher(&mut self) -> Result<&mut Self>;
pub async fn open_settings(&mut self) -> Result<&mut Self>;
pub async fn select_provider(&mut self, name: &str) -> Result<&mut Self>;
pub async fn enter_key(&mut self, key: &str) -> Result<&mut Self>;
pub async fn select_model(&mut self, name: &str) -> Result<&mut Self>;
pub async fn save(&mut self) -> Result<&mut Self>;
pub async fn complete_onboarding_with_mock(&mut self) -> Result<&mut Self>;
pub async fn expect_response(&mut self, msg: &str) -> Result<&mut Self>;
pub async fn expect_selected_row(&mut self, pattern: &str) -> Result<&mut Self>;
pub async fn request_tool_permission(&mut self, prompt: &str) -> Result<&mut Self>;
pub async fn allow_permission_always(&mut self) -> Result<&mut Self>;
pub async fn allow_permission_once(&mut self) -> Result<&mut Self>;
pub async fn deny_permission(&mut self) -> Result<&mut Self>;
pub async fn quit(&mut self) -> Result<&mut Self>;
pub async fn expect_text(&mut self, pattern: &str) -> Result<&mut Self>;
pub async fn expect_text_timeout(&mut self, pattern: &str, timeout: Duration) -> Result<&mut Self>;
pub async fn expect_no_text(&mut self, pattern: &str) -> Result<&mut Self>;
pub async fn expect_no_text_timeout(&mut self, pattern: &str, timeout: Duration) -> Result<&mut Self>;
pub async fn wait_for_exit(&mut self, timeout: Duration) -> Result<()>;
pub async fn ensure_alive(&mut self) -> Result<&mut Self>;
pub async fn capture(&mut self) -> Result<String>;
pub async fn wait_for_idle(&mut self) -> Result<()>;
pub async fn wait_for_text(&mut self, pattern: &str) -> Result<()>;
pub async fn run_with_timeout<F, T>(&mut self, future: F) -> Result<T>;
```

## Fixture resolution (`src/fixtures.rs`)

```rust
pub fn fixture_path(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("fixtures")
        .join(name)
}
```

Macro that panics if the fixture is missing:

```rust
let path = runie_tests::fixture_path!("openai/opencode_go_deepseek_v4_flash_simple.sse");
```

## Timeout configuration (`src/app_test.rs`)

```rust
#[derive(Debug, Clone)]
pub struct TimeoutConfig {
    pub startup: Duration,   // default 10s
    pub response: Duration,  // default 5s
    pub idle: Duration,      // default 1s
    pub dialog: Duration,    // default 3s
    pub build: Duration,     // default 300s
}
```

Constants:

```rust
pub const TEST_TIMEOUT: Duration = Duration::from_secs(60);
pub const SHORT_TIMEOUT: Duration = Duration::from_secs(5);
pub const MEDIUM_TIMEOUT: Duration = Duration::from_secs(15);
pub const LONG_TIMEOUT: Duration = Duration::from_secs(30);
pub const VERY_LONG_TIMEOUT: Duration = Duration::from_secs(60);
```

## Environment setup

`CliTest::assert()` and `TuiTest::capture_pane()` create a temp `HOME` with
`~/.runie/config.toml` generated from `TestConfig::replay()` and set:

```text
HOME=<temp_dir>
RUNIE_TEST_DATA_DIR=<temp_dir>
RUNIE_REPLAY_FIXTURES=<fixture_paths>
RUNIE_REPLAY_PROTOCOL=<openai|anthropic> (when .protocol() is used)
XDG_CONFIG_HOME=
XDG_DATA_HOME=
XDG_CACHE_HOME=
XDG_STATE_HOME=
```

Real API-key env vars are removed or blanked:
`ANTHROPIC_API_KEY`, `OPENAI_API_KEY`, `GOOGLE_API_KEY`, `DEEPSEEK_API_KEY`.

## Predicate helpers (`src/predicates.rs`)

```rust
pub trait Predicate<T: ?Sized>: Debug {
    fn matches(&self, value: &T) -> bool;
}

pub fn contains<S: Into<String>>(text: S) -> Contains;
pub fn not_contains<S: Into<String>>(text: S) -> NotContains;
pub fn matches<S: AsRef<str>>(pattern: S) -> Matches;
pub fn and<A, B>(a: A, b: B) -> And<A, B>;
pub fn or<A, B>(a: A, b: B) -> Or<A, B>;

pub struct ExitCode(pub i32);
pub struct Success; // exit code 0
pub struct Failure; // non-zero exit code
```

## Files in runie-tests

```text
runie-tests/src/
├── lib.rs          # prelude re-exports
├── cli.rs          # CliTest, CliAssert
├── tui.rs          # TuiTest, TuiAssert, tmux helpers
├── fixtures.rs     # fixture_path() and fixture_path! macro
├── predicates.rs   # Predicate trait and helpers
├── keys.rs         # Key constants
├── app_test.rs     # AppTest tmux harness, TimeoutConfig, binary discovery
├── test_config.rs  # TestConfig / ProviderConfig builders
└── prelude.rs      # Convenience re-exports
```

## Relationship to runie

- No imports from `runie_*` crates.
- No shared code with runie.
- Only contract is the subprocess CLI, `RUNIE_REPLAY_*` env vars, and the
  `.sse` file format consumed by `runie/crates/runie-provider/src/replay.rs`.

## Acceptance checklist

- [x] A Rust developer can write a single-turn CLI test in <8 lines.
- [x] A multi-turn CLI test requires only adding fixtures to `.fixtures([...])`.
- [x] A TUI tmux test can assert on captured pane text.
- [x] The DSL builds without any runie crate dependency.
- [x] Tests are deterministic and require no API keys.
