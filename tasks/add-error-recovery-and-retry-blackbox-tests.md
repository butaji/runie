# Add error recovery and retry behavior black-box tests

## Objective

Add black-box tests that prove Runie recovers from transient provider errors
and retries correctly. Cover exponential backoff, `retry-after` header respect,
retryable vs non-retryable classification, and the ability to continue a
session after failure.

## Why this matters

Production users hit rate limits and server errors. A public-ready tool must
retry transient failures automatically, surface non-retryable failures clearly,
and never leave the session in a stuck state.

## Research basis

- Opencode tests retry delay against `retry-after` and `retry-after-ms` headers
  (`~/Code/agents/opencode/packages/opencode/test/session/retry.test.ts`).
- Codex tests recovery after a 500 response followed by a successful turn
  (`~/Code/agents/codex/codex-rs/core/tests/suite/stream_error_allows_next_turn.rs`).
- Runie's `runie/crates/runie-provider/src/retry.rs` already implements
  exponential backoff and typed error classification.

## Test scenarios

### Retry with exponential backoff

```rust
#[tokio::test]
async fn cli_retries_rate_limit_with_exponential_backoff() {
    test_cli()
        .fixtures([
            "openai/rate_limit_error.sse",
            "openai/rate_limit_error.sse",
            "openai/glm_5_simple.sse", // success on third attempt
        ])
        .args(["print", "hello"])
        .assert()
        .await
        .unwrap()
        .success()
        .stdout(contains("expected success content"));
}
```

This test requires the replay provider to cycle through fixtures and the retry
layer to keep attempting until the success fixture is reached.

### Retry-after header respect

```rust
#[tokio::test]
async fn cli_respects_retry_after_header() {
    test_cli()
        .fixtures([
            "openai/rate_limit_error.sse", // contains retry_after: 1
            "openai/glm_5_simple.sse",
        ])
        .args(["print", "hello"])
        .assert()
        .await
        .unwrap()
        .success();
    // Wall-clock time should be at least 1s; use a loose assertion or
    // instrumented replay log.
}
```

### Non-retryable errors do not loop

```rust
#[tokio::test]
async fn cli_does_not_retry_auth_error() {
    test_cli()
        .fixture("openai/invalid_api_key.sse")
        .args(["print", "hello"])
        .assert()
        .await
        .unwrap()
        .failure();
    // Assert fixture was consumed exactly once (requires replay counter
    // observable in test, or rely on fast failure + timeout).
}
```

### TUI allows next turn after transient error

```rust
#[tokio::test]
async fn tui_allows_next_turn_after_rate_limit() {
    test_tui()
        .fixture("openai/rate_limit_error.sse")
        .type_keys("first")
        .submit()
        .wait_for_idle(Duration::from_millis(500))
        .assert_pane(contains("rate limit"))
        .type_keys("second")
        .submit()
        .wait_for_idle(Duration::from_millis(500))
        .capture_pane()
        .await
        .unwrap()
        .assert(contains("rate limit").and(contains("second")));
}
```

### Context-length errors are fatal and readable

```rust
#[tokio::test]
async fn cli_context_length_error_is_not_retried() {
    test_cli()
        .fixture("openai/context_length_exceeded.sse")
        .args(["print", "hello"])
        .assert()
        .await
        .unwrap()
        .failure()
        .stderr(contains("context").or(contains("too long")));
}
```

## Required runie changes

- Expose replay fixture cycling so retry tests can supply a fail-then-succeed
  sequence.
- Ensure `retry-after`/`retry-after-ms` values from error payloads are passed
  to the backoff logic.
- Ensure non-retryable errors (401, 403, context length, model not found) fail
  fast without retry.

## Dependencies

- `add_openai_error_sse_fixtures`
- `add_anthropic_error_sse_fixtures`
- `wire_error_sse_fixtures_to_cli_replay_tests`
- `wire_error_sse_fixtures_to_tui_replay_tests`
- `black_box_replay_testing`

## Acceptance checklist

- [ ] Retry test proves transient errors are retried and eventually succeed.
- [ ] Retry-after test proves backoff respects provider hints.
- [ ] Non-retryable auth/context/model errors fail on first attempt.
- [ ] TUI recovery test proves the app accepts input after an error.
- [ ] All new tests pass with `--test-threads=2`.
