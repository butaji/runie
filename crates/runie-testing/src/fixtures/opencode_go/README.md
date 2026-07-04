# OpenCode Go replay fixtures

These fixtures were recorded from the [OpenCode Go](https://opencode.ai/docs/go/)
model gateway (`https://opencode.ai/zen/go/v1`) using the recorder at
`scripts/record_opencode_go.py`.

## Models covered

The OpenCode Go catalog exposes two API families:

- **OpenAI-compatible** (`/v1/chat/completions`): `deepseek-v4-pro`,
  `deepseek-v4-flash`, `glm-5`, `glm-5.1`, `glm-5.2`, `kimi-k2.5`,
  `kimi-k2.6`, `kimi-k2.7-code`, `mimo-v2.5`, `mimo-v2.5-pro`.
- **Anthropic-compatible** (`/v1/messages`): `minimax-m2.5`, `minimax-m2.7`,
  `minimax-m3`, `qwen3.5-plus`, `qwen3.6-plus`, `qwen3.7-max`, `qwen3.7-plus`.

The following models are listed by `/v1/models` but returned errors during
recording and are not represented here:

- `hy3-preview` — `400 Bad Request`
- `mimo-v2-omni` — `502 Bad Gateway`
- `mimo-v2-pro` — `502 Bad Gateway`

## Scenarios

Every successfully recorded model has at least:

- `simple` — "Reply with only the word 'ok'."
- `tool` — "What is the weather in Paris?" with a single `get_weather` tool.

Representative models also have:

- `multi_tool` — "What is the weather in Paris and Berlin?"
- `reasoning` — "What is 9 times 7? Show your reasoning briefly."

In addition, multi-turn conversation fixtures are recorded for representative
models. Each turn is stored as a separate `.sse` file named
`opencode_go_<model>_multiturn_<scenario>_turn<N>.sse`:

- `math_chain` — "What is 2 + 2?" / "Multiply that by 3."
- `weather_chain` — "What is the weather in Paris?" / "What about Berlin?"
- `read_summarize_followup` — read file, summarize, then answer follow-up
- `reasoning_followup` — reasoning answer then follow-up on the result
- `multi_tool_then_compare` — parallel tool calls then comparison question
- `clarification` — vague request, model asks clarification, then answers

Representative OpenAI-compatible models: `deepseek-v4-pro`,
`deepseek-v4-flash`, `glm-5.2`, `kimi-k2.6`, `mimo-v2.5`.
Representative Anthropic-compatible models: `minimax-m3`, `minimax-m2.7`,
`qwen3.7-max`, `qwen3.7-plus`.

## File layout

- `../openai/opencode_go_*.sse` — OpenAI-compatible `/v1/chat/completions`
  traces.
- `../anthropic/opencode_go_*.sse` — Anthropic-compatible `/v1/messages`
  traces.
- `../../../target/tmp/opencode-go-raw/` — Raw, unsanitized captures and the
  recording manifest (`manifest.json`).

## Sanitization

All fixtures are deterministic:

- OpenAI completion ids are replaced with `chatcmpl-opencode-go-fixture`.
- Anthropic message/content-block ids are replaced with fixture ids.
- `created` timestamps are zeroed.
- `system_fingerprint` is normalized.
- Ping cost values are zeroed.
- Real model names are preserved so tests can assert on them.

## Replaying

OpenAI-compatible fixtures are replayed through
`runie_provider::openai::stream::replay_sse`.
Anthropic-compatible fixtures are replayed through
`runie_provider::anthropic::replay_anthropic_sse`.

See `crates/runie-provider/tests/opencode_go_openai_replay.rs` and
`crates/runie-provider/tests/opencode_go_anthropic_replay.rs`.

## Re-recording

Single-turn fixtures:

```bash
export OPENCODE_GO_API_KEY=sk-...
python3 scripts/record_opencode_go.py
```

Multi-turn fixtures:

```bash
export OPENCODE_GO_API_KEY=sk-...
python3 scripts/record_opencode_go_multiturn.py
```

Raw captures land in `target/tmp/opencode-go-raw/` (single-turn) or
`target/tmp/opencode-go-raw/multiturn/` (multi-turn). Sanitized fixtures are
written to the `openai/` and `anthropic/` directories above. Inspect the
recording manifests for the full mapping.

## Black-box testing

These fixtures can drive the real `runie-cli` and `runie-tui` binaries through
the replay provider. See `docs/BlackBoxTesting.md` for details.

Quick examples:

```bash
# CLI simple text
RUNIE_REPLAY_FIXTURES=../openai/opencode_go_deepseek_v4_flash_simple.sse \
  cargo run -p runie-cli -- print "say ok"

# CLI tool call
RUNIE_REPLAY_FIXTURES=../openai/opencode_go_deepseek_v4_flash_tool.sse \
  cargo run -p runie-cli -- json --model opencode-go/deepseek-v4-flash "weather in Paris"

# TUI
RUNIE_REPLAY_FIXTURES=../openai/opencode_go_kimi_k2_6_simple.sse \
  cargo run -p runie-tui -- --provider opencode-go --model kimi-k2.6

# Anthropic protocol
RUNIE_REPLAY_PROTOCOL=anthropic \
RUNIE_REPLAY_FIXTURES=../anthropic/opencode_go_minimax_m3_simple.sse \
  cargo run -p runie-cli -- print "say ok"

# Multi-turn conversation
RUNIE_REPLAY_FIXTURES="\
../openai/opencode_go_deepseek_v4_pro_multiturn_weather_chain_turn1.sse,\
../openai/opencode_go_deepseek_v4_pro_multiturn_weather_chain_turn2.sse" \
  cargo run -p runie-cli -- print "What is the weather in Paris?" "What about Berlin?"
```
