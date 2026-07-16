# Add Anthropic-compatible error SSE fixtures

## Objective

Add recorded SSE fixtures under `runie-tests/fixtures/anthropic/` that simulate
provider-side error events for the Anthropic protocol (used by MiniMax and Qwen
models in this suite). These fixtures pair with the OpenAI error fixtures to
ensure Runie handles errors for every supported protocol.

## Why this matters

The black-box suite already replays Anthropic-protocol fixtures for happy-path
scenarios (simple, tool, multi-tool, reasoning, multi-turn). Failure paths must
have equal coverage before the suite can be considered production-quality.

## Fixtures to add

| Fixture | Scenario | Expected retryable |
|---------|----------|-------------------|
| `fixtures/anthropic/rate_limit_error.sse` | Rate limit with `retry_after: 30` | ✅ yes |
| `fixtures/anthropic/server_error.sse` | Provider server error chunk | ✅ yes |
| `fixtures/anthropic/stream_error_mid_response.sse` | Normal content chunks followed by `error:` chunk | ✅ yes |
| `fixtures/anthropic/context_length_exceeded.sse` | Context window exceeded | ❌ no |
| `fixtures/anthropic/invalid_api_key.sse` | Authentication failure inside SSE | ❌ no |

## Fixture format rules

- Follow the same `data:` line format as existing `opencode_go_minimax_*.sse`
  and `opencode_go_qwen_*.sse` fixtures.
- Use Anthropic-style error payloads (`type: error`, `error: {type, message}`).
- Add exactly one `error:` line per fixture.
- Omit the trailing `data: [DONE]` when the fixture ends on the error event.

## Example `rate_limit_error.sse`

```text
data: {"type":"message_start","message":{"id":"msg-error","type":"message","role":"assistant","model":"minimax-m3","content":[],"stop_reason":null,"stop_sequence":null,"usage":{"input_tokens":10,"output_tokens":0}}}
error: {"type":"error","error":{"type":"rate_limit_error","message":"Rate limit exceeded. Please try again in 30s."}}
```

## Required runie changes

- Ensure `runie-cli print` exits non-zero with a readable error message for
  Anthropic-protocol error events.
- Ensure `runie-tui` shows an error banner or status-bar error state.
- Ensure no API key value is printed in error output.

## Dependencies

- `black_box_replay_testing`
- `cli_replay_dsl`
- `tui_dsl_polling_waits`
- `add_openai_error_sse_fixtures` (follow the same pattern)

## Acceptance checklist

- [ ] All 5 fixtures above exist under `fixtures/anthropic/`.
- [ ] Each fixture parses without panic when passed through the Anthropic replay
      parser.
- [ ] Each fixture is referenced by at least one black-box test.
- [ ] No secret values appear in any fixture.
