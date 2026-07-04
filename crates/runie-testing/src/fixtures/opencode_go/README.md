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

```bash
export OPENCODE_GO_API_KEY=sk-...
python3 scripts/record_opencode_go.py
```

Raw captures land in `target/tmp/opencode-go-raw/` and sanitized fixtures are
written to the `openai/` and `anthropic/` directories above. Inspect
`target/tmp/opencode-go-raw/manifest.json` for the full mapping.
