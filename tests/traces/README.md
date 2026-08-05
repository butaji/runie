# OpenCode Go replay fixtures

These fixtures were recorded from the [OpenCode Go](https://opencode.ai/docs/go/)
model gateway (`https://opencode.ai/zen/go/v1`) using the recorders in
[`scripts/`](scripts/).

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

## Error and HTTP-status fixtures

Both protocol directories contain SSE error fixtures for failure paths:

- `rate_limit_error.sse` — provider 429/rate-limit during the SSE stream.
- `rate_limit_error_ms.sse` — OpenAI-specific rate-limit with Microsoft-style headers (openai only).
- `server_error.sse` — provider 500/server-error during the SSE stream.
- `stream_error_mid_response.sse` — malformed/error chunk mid-stream.
- `context_length_exceeded.sse` — context-length exceeded error.
- `invalid_api_key.sse` — invalid API key error.
- `model_not_found.sse` — model not found error (openai only).

HTTP-status fixtures simulate failures before the SSE stream starts:

- `status_401_unauthorized.sse`
- `status_403_forbidden.sse` (openai only)
- `status_429_rate_limit.sse`
- `status_500_server_error.sse`

These fixtures drive the error/rate-limit black-box tests in
`tests/cli_replay.rs`, `tests/tui_replay_conversations.rs`, and
`tests/error_recovery.rs`.

## File layout

- `openai/opencode_go_*.sse` — OpenAI-compatible `/v1/chat/completions`
  traces.
- `anthropic/opencode_go_*.sse` — Anthropic-compatible `/v1/messages`
  traces.
- `runie/target/tmp/opencode-go-raw/` — Raw, unsanitized captures and the
  recording manifest (`manifest.json`). This path is inside the `runie`
  submodule because raw captures are gitignored and not committed.

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

Black-box tests that exercise the real `runie-cli` and `runie-tui` binaries
live in the parent `runie-tests` repo; see `docs/black-box-replay-testing.md`.

## Re-recording

From the `runie-tests` repo root:

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

Raw captures land in `runie/target/tmp/opencode-go-raw/` (single-turn) or
`runie/target/tmp/opencode-go-raw/multiturn/` (multi-turn). Sanitized fixtures
are written to `fixtures/openai/` and `fixtures/anthropic/`. Inspect the
recording manifests for the full mapping.

## Quick CLI replay examples

Run these from the `runie` submodule directory:

```bash
# CLI simple text
RUNIE_REPLAY_FIXTURES=../fixtures/openai/opencode_go_deepseek_v4_flash_simple.sse \
  cargo run -p runie-cli -- print "say ok"

# CLI tool call
RUNIE_REPLAY_FIXTURES=../fixtures/openai/opencode_go_deepseek_v4_flash_tool.sse \
  cargo run -p runie-cli -- json --model opencode-go/deepseek-v4-flash "weather in Paris"

# TUI
RUNIE_REPLAY_FIXTURES=../fixtures/openai/opencode_go_kimi_k2_6_simple.sse \
  cargo run -p runie-tui -- --provider opencode-go --model kimi-k2.6

# Anthropic protocol
RUNIE_REPLAY_PROTOCOL=anthropic \
RUNIE_REPLAY_FIXTURES=../fixtures/anthropic/opencode_go_minimax_m3_simple.sse \
  cargo run -p runie-cli -- print "say ok"

# Multi-turn conversation
RUNIE_REPLAY_FIXTURES="\
../fixtures/openai/opencode_go_deepseek_v4_pro_multiturn_weather_chain_turn1.sse,\
../fixtures/openai/opencode_go_deepseek_v4_pro_multiturn_weather_chain_turn2.sse" \
  cargo run -p runie-cli -- print "What is the weather in Paris?" "What about Berlin?"
```

## Google Gemini fixtures

`fixtures/gemini/` holds traces recorded against Gemini's OpenAI-compatible
endpoint (`https://generativelanguage.googleapis.com/v1beta/openai`) with
`gemini-3.1-flash-lite`, using `scripts/record_gemini.py`:

```bash
export GEMINI_API_KEY=...
python3 scripts/record_gemini.py
```

Scenarios: `simple`, `tool`, `multi_tool`, `reasoning`, a two-turn
`multiturn_weather_chain`, plus error traces (`gemini_status_400_invalid_
api_key.sse`, `gemini_model_not_found.sse`) stored as `# HTTP <code>` fixtures.

Gemini quirks pinned by these fixtures:

- Tool-call deltas omit the `index` field and carry
  `extra_content.google.thought_signature` (scrubbed to a placeholder).
- Tool turns end with `finish_reason: "stop"` (not `"tool_calls"`).
- Error bodies are JSON arrays (`[{"error": {...}}]`), and an invalid key is
  rejected with `400 INVALID_ARGUMENT`, not 401.
- Gemini 3 emits no `reasoning_content` — thinking is not exposed on the
  OpenAI-compatible surface, only opaque thought signatures.

Raw captures land in `runie/target/tmp/gemini-raw/` (gitignored). The
black-box tests live in `tests/cli_replay_gemini.rs`.

## Kimi Code fixtures

`fixtures/kimi-code/` holds traces recorded against the Kimi
coding-subscription endpoint (`https://api.kimi.com/coding/v1`,
OpenAI-compatible; NOT the Moonshot platform API) with
`kimi-for-coding-highspeed`, using `scripts/record_kimi_code.py`:

```bash
export KIMI_API_KEY=...
python3 scripts/record_kimi_code.py
```

Scenarios: `simple`, `tool`, `multi_tool`, `reasoning`, a two-turn
`multiturn_weather_chain`, plus error traces
(`kimi_code_status_401_invalid_api_key.sse`,
`kimi_code_status_404_model_not_found.sse`) stored as `# HTTP <code>`
fixtures.

Kimi Code quirks pinned by these fixtures:

- Both models (`kimi-for-coding`, `kimi-for-coding-highspeed`) are
  always-thinking: every response streams `reasoning_content` deltas before
  `content`, and reasoning consumes the completion token budget (a too-small
  budget yields empty content with `finish_reason: "length"`).
- Error envelopes are plain OpenAI-shaped objects
  (`{"error": {"message", "type"}}`); an invalid key is rejected with
  `401 invalid_authentication_error`.
- Chat completions silently accepts ANY model string and still answers
  (200) — the model-not-found 404 (`resource_not_found_error`) only exists
  on `GET /models/{model}`, which is where that trace is recorded.
- Chunks carry a per-deployment `system_fingerprint`, normalized to
  `fp-kimi-code-fixture` during sanitization.

Raw captures land in `runie/target/tmp/kimi-code-raw/` (gitignored). The
black-box tests live in `tests/cli_replay_kimi_code.rs`.
