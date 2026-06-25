# Replace custom SSE framing with `reqwest-eventsource`

**Status**: todo
**Milestone**: R4
**Category**: Provider
**Priority**: P1

**Depends on**: none
**Blocks**: none

## Summary

Delete `crates/runie-provider/src/framing.rs` and consume provider Server-Sent Event streams using `reqwest-eventsource`.

## Acceptance Criteria

- `reqwest-eventsource` is added to `runie-provider` dependencies.
- Custom SSE framing parser is removed.
- OpenAI-compatible streaming continues to emit correct `ProviderEvent`s.
- Reconnection/backoff is delegated to the crate or composed with the retry layer.
- `cargo check --workspace` is green with no new warnings.

## Tests

- **Layer 1**: Unit tests for event parsing from byte chunks.
- **Layer 4**: Provider-replay test with chunked SSE fixtures covering split events and retries.
