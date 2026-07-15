# Add OpenAI-compatible error SSE fixtures

## Objective

Add recorded SSE fixtures under `runie-tests/fixtures/openai/` that simulate
provider-side error events delivered inside an otherwise-normal SSE stream.
These fixtures drive CLI and TUI black-box tests without network calls or API
keys.

## Why this matters

Runie's OpenAI stream parser (`runie/crates/runie-provider/src/openai/stream.rs`)
already supports fixture error lines of the form:

```text
error: {"type":"rate_limit_error","message":"..."}
```

We must exercise this path in black-box tests so users see clear, non-crashing
error handling for retryable and non-retryable provider errors.

## Fixtures to add

| Fixture | Scenario | Expected retryable |
|---------|----------|-------------------|
| `fixtures/openai/rate_limit_error.sse` | Rate limit with `retry_after: 30` | ✅ yes |
| `fixtures/openai/rate_limit_error_ms.sse` | Rate limit with `retry_after_ms: 1500` | ✅ yes |
| `fixtures/openai/server_error.sse` | Provider `server_error` chunk | ✅ yes |
| `fixtures/openai/stream_error_mid_response.sse` | Normal content chunks followed by `error:` chunk | ✅ yes |
| `fixtures/openai/context_length_exceeded.sse` | Context window exceeded | ❌ no |
| `fixtures/openai/model_not_found.sse` | Requested model does not exist | ❌ no |
| `fixtures/openai/invalid_api_key.sse` | Authentication failure inside SSE | ❌ no |

## Fixture format rules

- Use the same `data:` line format as existing `opencode_go_*.sse` fixtures.
- Add exactly one `error:` line per fixture.
- Error JSON must match the shape consumed by `parse_error_value` in
  `runie/crates/runie-provider/src/openai/protocol.rs`.
- Include the trailing `data: [DONE]` only for fixtures that end normally; omit
  it for fixtures whose last event is the error chunk.

## Example `rate_limit_error.sse`

```text
data: {"id":"chatcmpl-error","object":"chat.completion.chunk","created":0,"model":"deepseek-v4-flash","choices":[{"index":0,"delta":{"role":"assistant","content":""},"finish_reason":null}]}
error: {"type":"rate_limit_error","message":"Rate limit exceeded. Please try again in 30s.","retry_after":30}
```

## Required runie changes

- Ensure `runie-cli print` exits non-zero and emits a readable error message
  when the stream yields an error event.
- Ensure `runie-tui` shows an error banner or status-bar error state instead of
  hanging.
- Ensure no API key value is printed in error output.

## Dependencies

- `black_box_replay_testing`
- `cli_replay_dsl`
- `tui_dsl_polling_waits`

## Acceptance checklist

- [ ] All 7 fixtures above exist under `fixtures/openai/`.
- [ ] Each fixture parses without panic when passed through `replay_sse`.
- [ ] Each fixture is referenced by at least one black-box test.
- [ ] No secret values appear in any fixture.
