# Wire error SSE fixtures to TUI replay tests

## Objective

Add TUI black-box tests in `tests/tui_replay_conversations.rs` that exercise
every error SSE fixture. Verify that `runie-tui` surfaces errors in the pane,
does not crash, and remains interactive for a follow-up turn.

## Why this matters

In the TUI, users cannot see a process exit code. Errors must appear in the
scrollback or status bar, and the app must recover enough to accept the next
message. This is a hard requirement for a public-facing terminal UI.

## Test scenarios

Add one `#[tokio::test]` per fixture group below using the TUI replay DSL.

### OpenAI protocol

```rust
#[tokio::test]
async fn tui_openai_rate_limit_error_renders() {
    test_tui()
        .fixture("openai/rate_limit_error.sse")
        .type_keys("hello")
        .submit()
        .wait_for_idle(Duration::from_millis(500))
        .capture_pane()
        .await
        .unwrap()
        .assert(contains("rate limit").and(contains("retry").or(contains("30s"))));
}
```

Repeat for:
- `openai/server_error.sse` — pane contains "server" or "error".
- `openai/stream_error_mid_response.sse` — pane contains error indication and
  does not falsely claim the response completed successfully.
- `openai/context_length_exceeded.sse` — pane contains "context" or "tokens".
- `openai/model_not_found.sse` — pane contains "model" or "not found".
- `openai/invalid_api_key.sse` — pane contains "invalid" or "authentication";
  assert pane does not contain the fixture key value.

### Anthropic protocol

Repeat the same shape for the 5 Anthropic-protocol error fixtures.

### Recovery after error

```rust
#[tokio::test]
async fn tui_recovers_after_openai_rate_limit_error() {
    test_tui()
        .fixture("openai/rate_limit_error.sse")
        .type_keys("first")
        .submit()
        .wait_for_idle(Duration::from_millis(500))
        .assert_pane(contains("rate limit"))
        // Second turn uses the same fixture again; app must still be interactive.
        .type_keys("second")
        .submit()
        .wait_for_idle(Duration::from_millis(500))
        .capture_pane()
        .await
        .unwrap()
        .assert(contains("rate limit"));
}
```

## Required runie changes

- `runie-tui` must render an error state in the pane or status bar when the
  provider stream yields an error event.
- The input box must remain usable after an error.
- The app must not panic or exit on error.

## Dependencies

- `add_openai_error_sse_fixtures`
- `add_anthropic_error_sse_fixtures`
- `tui_dsl_polling_waits`
- `black_box_replay_testing`

## Acceptance checklist

- [ ] Every OpenAI error fixture has a dedicated TUI test.
- [ ] Every Anthropic error fixture has a dedicated TUI test.
- [ ] At least one recovery test proves the TUI accepts input after an error.
- [ ] No test contains a real or fixture API key string in assertions.
- [ ] `cargo test --test tui_replay_conversations` passes after implementation.
