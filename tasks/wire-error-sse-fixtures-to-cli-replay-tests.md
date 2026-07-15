# Wire error SSE fixtures to CLI replay tests

## Objective

Add CLI black-box tests in `tests/cli_replay.rs` that exercise every error
SSE fixture added by `add-openai-error-sse-fixtures` and
`add-anthropic-error-sse-fixtures`.

## Why this matters

Users invoke `runie print` non-interactively. The CLI must surface provider
errors as readable, non-zero-exit failures without leaking secrets. Each error
fixture must be proven through the actual compiled binary.

## Test scenarios

Add one `#[tokio::test]` per fixture group below.

### OpenAI protocol

```rust
#[tokio::test]
async fn openai_rate_limit_error_renders() {
    test_cli()
        .fixture("openai/rate_limit_error.sse")
        .args(["print", "hello"])
        .assert()
        .await
        .unwrap()
        .failure()
        .stderr(contains("rate limit").and(contains("retry").or(contains("30s"))));
}
```

Repeat for:
- `openai/server_error.sse` — assert stderr contains "server" or "error".
- `openai/stream_error_mid_response.sse` — assert partial content is not emitted
  as final answer; stderr or stdout contains error indication.
- `openai/context_length_exceeded.sse` — assert non-zero exit and message
  mentions context/window/tokens.
- `openai/model_not_found.sse` — assert non-zero exit and message mentions model
  or "not found".
- `openai/invalid_api_key.sse` — assert non-zero exit, message mentions
  "authentication" or "invalid key", and stdout/stderr does not contain the
  fixture key value.

### Anthropic protocol

Repeat the same shape for the Anthropic-protocol error fixtures, asserting on
Anthropic-style error text:

- `anthropic/rate_limit_error.sse` — assert stderr contains "rate limit" or "retry".
- `anthropic/server_error.sse` — assert stderr contains "server" or "error".
- `anthropic/stream_error_mid_response.sse` — assert non-zero exit and stderr or
  stdout contains "error" or "server".
- `anthropic/context_length_exceeded.sse` — assert non-zero exit and message
  mentions context/window/tokens.
- `anthropic/invalid_api_key.sse` — assert non-zero exit, message mentions
  "authentication" or "invalid key", and stdout/stderr does not contain the
  fixture key value.

### Aggregate fixture test

```rust
#[tokio::test]
async fn all_error_fixtures_exit_nonzero() {
    for fixture in [
        "openai/rate_limit_error.sse",
        "openai/server_error.sse",
        "openai/context_length_exceeded.sse",
        "openai/model_not_found.sse",
        "openai/invalid_api_key.sse",
        "anthropic/rate_limit_error.sse",
        "anthropic/server_error.sse",
        "anthropic/stream_error_mid_response.sse",
        "anthropic/context_length_exceeded.sse",
        "anthropic/invalid_api_key.sse",
    ] {
        test_cli()
            .fixture(fixture)
            .args(["print", "hello"])
            .assert()
            .await
            .unwrap()
            .failure();
    }
}
```

## Required runie changes

- `runie-cli print` must return a non-zero exit code when the provider stream
  yields an error event.
- `runie-cli print` must write the error message to stderr (or stdout if that
  is the current behavior), but must never print the API key.

## Dependencies

- `add_openai_error_sse_fixtures`
- `add_anthropic_error_sse_fixtures`
- `cli_replay_dsl`
- `black_box_replay_testing`

## Acceptance checklist

- [x] Every OpenAI error fixture has a dedicated CLI test.
- [x] Every Anthropic error fixture has a dedicated CLI test.
- [x] Aggregate test loops all error fixtures and asserts non-zero exit.
- [x] No test contains a real or fixture API key string in assertions.
- [x] `cargo test --test cli_replay` passes after implementation.
