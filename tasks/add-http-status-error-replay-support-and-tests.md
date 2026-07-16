# Add HTTP-status error replay support and black-box tests

## Objective

Extend the replay provider and test harness so black-box tests can simulate
HTTP-level provider failures (429, 500, 502, 503, 401, 403) without network
calls. Then add CLI and TUI tests that prove Runie handles each status.

## Why this matters

SSE fixtures only cover errors that arrive *inside* a 200 OK stream. Real
providers also fail at the HTTP boundary with non-2xx status codes. Runie's
retry classifier (`runie/crates/runie-provider/src/retry.rs`) already treats
429/500/502/503 as retryable and 401/403 as non-retryable, but this logic is
not exercised by the current black-box suite.

## Required runie changes

1. **Replay provider extension**
   - Add a new replay fixture format or metadata that signals an HTTP status
     error instead of SSE content.
   - Option A: fixture file extension (e.g., `.http429` or `.status429`).
   - Option B: fixture path convention (e.g.,
     `fixtures/openai/status_429_rate_limit.sse` with a header line).
   - Option C: environment variable override (e.g.,
     `RUNIE_REPLAY_STATUS=429`).
   - Recommended: Option A with a small per-fixture metadata file, because it
     keeps tests declarative and hermetic.

2. **Status-code-to-provider-error mapping**
   - Ensure the replay path feeds the status code into
     `ProviderError::classify_http_status` so the same retry logic applies.

3. **CLI/TUI behavior**
   - `runie-cli print` exits non-zero for 4xx/5xx statuses.
   - `runie-tui` shows an error banner and remains interactive.

## Fixtures to add

| Fixture | Status | Retryable |
|---------|--------|-----------|
| `fixtures/openai/status_429_rate_limit.sse` | 429 Too Many Requests | yes |
| `fixtures/openai/status_500_server_error.sse` | 500 Internal Server Error | yes |
| `fixtures/openai/status_502_bad_gateway.sse` | 502 Bad Gateway | yes |
| `fixtures/openai/status_503_service_unavailable.sse` | 503 Service Unavailable | yes |
| `fixtures/openai/status_401_unauthorized.sse` | 401 Unauthorized | no |
| `fixtures/openai/status_403_forbidden.sse` | 403 Forbidden | no |
| `fixtures/anthropic/status_429_rate_limit.sse` | 429 Too Many Requests | yes |
| `fixtures/anthropic/status_500_server_error.sse` | 500 Internal Server Error | yes |
| `fixtures/anthropic/status_401_unauthorized.sse` | 401 Unauthorized | no |

## Test scenarios

### CLI

```rust
#[tokio::test]
async fn cli_replays_429_rate_limit() {
    test_cli()
        .fixture("openai/status_429_rate_limit.sse")
        .args(["print", "hello"])
        .assert()
        .await
        .unwrap()
        .failure()
        .stderr(contains("rate limit").or(contains("429")));
}
```

Add equivalent tests for 500, 502, 503, 401, 403, and the Anthropic variants.

### TUI

```rust
#[tokio::test]
async fn tui_renders_429_rate_limit() {
    test_tui()
        .fixture("openai/status_429_rate_limit.sse")
        .type_keys("hello")
        .submit()
        .wait_for_idle(Duration::from_millis(500))
        .capture_pane()
        .await
        .unwrap()
        .assert(contains("rate limit").or(contains("429")));
}
```

## Dependencies

- `black_box_replay_testing`
- `cli_replay_dsl`
- `tui_dsl_polling_waits`
- `add_openai_error_sse_fixtures` (share error-message assertion patterns)

## Acceptance checklist

- [ ] Replay provider can return HTTP status errors for both protocols.
- [ ] Each status code above has a fixture.
- [ ] Each fixture has a CLI test asserting non-zero exit and readable message.
- [ ] Each fixture has a TUI test asserting pane message and app survival.
- [ ] 429/500/502/503 tests verify retry attempts occur (via log inspection or
      fixture sequence), while 401/403 tests verify no retry.
