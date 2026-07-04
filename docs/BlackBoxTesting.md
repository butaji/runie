# Black-box replay testing

Runie can run the real `runie-cli` and `runie-tui` binaries against recorded
SSE fixtures instead of live model APIs. This is useful for deterministic
black-box tests, demos, and debugging without spending quota or needing API
keys.

## How it works

When `RUNIE_REPLAY_FIXTURES` is set, `BuiltProviderFactory` loads the listed
`.sse` files and returns a `ReplayProvider`. The provider replays the SSE
stream as if it came from a live model, emitting the same `ProviderEvent`
sequence. No API key, base URL, or network call is involved.

## Running the CLI in replay mode

```bash
RUNIE_REPLAY_FIXTURES=crates/runie-testing/src/fixtures/openai/opencode_go_deepseek_v4_flash_simple.sse \
  cargo run -p runie-cli -- print "say ok"
```

For Anthropic-compatible fixtures, set the protocol explicitly:

```bash
RUNIE_REPLAY_PROTOCOL=anthropic \
RUNIE_REPLAY_FIXTURES=crates/runie-testing/src/fixtures/anthropic/opencode_go_minimax_m3_simple.sse \
  cargo run -p runie-cli -- print "say ok"
```

Multiple fixtures can be chained for multi-turn tests:

```bash
RUNIE_REPLAY_FIXTURES="\
crates/runie-testing/src/fixtures/openai/opencode_go_deepseek_v4_flash_simple.sse,\
crates/runie-testing/src/fixtures/openai/opencode_go_deepseek_v4_flash_tool.sse" \
  cargo run -p runie-cli -- print "turn one" "turn two"
```

### Multi-turn fixtures

OpenCode Go multi-turn conversations are recorded per-turn. Each turn is a
separate `.sse` file named `*_multiturn_<scenario>_turn<N>.sse`. Chain them in
order to replay a full conversation:

```bash
RUNIE_REPLAY_FIXTURES="\
crates/runie-testing/src/fixtures/openai/opencode_go_deepseek_v4_pro_multiturn_weather_chain_turn1.sse,\
crates/runie-testing/src/fixtures/openai/opencode_go_deepseek_v4_pro_multiturn_weather_chain_turn2.sse" \
  cargo run -p runie-cli -- print "What is the weather in Paris?" "What about Berlin?"
```

Recorded multi-turn scenarios include:

- `math_chain` — follow-up math question
- `weather_chain` — tool call followed by another tool call
- `read_summarize_followup` — read file, summarize, answer follow-up
- `reasoning_followup` — reasoning answer followed by another reasoning step
- `multi_tool_then_compare` — parallel tool calls then comparison question
- `clarification` — vague request, model asks clarification, then answers

## Running the TUI in replay mode

```bash
RUNIE_REPLAY_FIXTURES=crates/runie-testing/src/fixtures/openai/opencode_go_kimi_k2_6_simple.sse \
  cargo run -p runie-tui -- --provider opencode-go --model kimi-k2.6
```

The TUI will behave exactly as if the model produced the recorded response.
You can interact with it, inspect the rendered output, and verify that tool
calls, reasoning blocks, and streaming deltas are displayed correctly.

## Tool-call fixtures

Fixtures that emit `ToolCallStart` need deterministic tool outputs. The replay
provider ships with a small static registry of canned responses for common
read-only tools:

| Tool name | Canned response |
|-----------|-----------------|
| `get_weather` | `{"temperature":"22","unit":"celsius","condition":"sunny"}` |
| `list_dir` | `{"entries":["README.md","Cargo.toml"]}` |
| `read_file` | `{"content":"# Example\n"}` |

For custom tool responses, set `RUNIE_REPLAY_TOOL_<NAME>`:

```bash
RUNIE_REPLAY_TOOL_GET_WEATHER='{"temperature":"-5","unit":"celsius"}' \
RUNIE_REPLAY_FIXTURES=... \
  cargo run -p runie-cli -- print "weather in Moscow"
```

## Writing black-box tests

### CLI tests

Use `assert_cmd` to spawn the compiled binary and assert on stdout/stderr:

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
        .stdout(predicates::str::contains("ok"));
}
```

### TUI tests

For deterministic CI tests, drive `TuiRuntime` with a `TestBackend` and set
`RUNIE_REPLAY_FIXTURES` via `with_env`. This stays inside the Rust test
process and avoids terminal quirks.

For true end-to-end terminal interaction tests, spawn the compiled `runie-tui`
binary inside `tmux` or using `expect`, feed keystrokes, and assert on screen
capture. These are slower but catch real terminal integration issues.

## Fixture catalog

Recorded OpenCode Go fixtures live in:

- `crates/runie-testing/src/fixtures/openai/` — OpenAI-compatible traces.
- `crates/runie-testing/src/fixtures/anthropic/` — Anthropic-compatible traces.

See `crates/runie-testing/src/fixtures/opencode_go/README.md` for the full
catalog of models and scenarios.

## Adding new fixtures

1. Record a new trace with `scripts/record_opencode_go.py` or capture it
   manually.
2. Sanitize non-deterministic fields (ids, timestamps, fingerprints, costs).
3. Save the `.sse` file under `crates/runie-testing/src/fixtures/<protocol>/`.
4. Reference it in a black-box test via `RUNIE_REPLAY_FIXTURES`.
