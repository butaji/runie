# Replace custom retry module with `backon`

**Status**: todo
**Milestone**: R1
**Category**: Provider / Network
**Priority**: P0

**Depends on**: none
**Blocks**: none

## Description

`crates/runie-provider/src/retry.rs` implements a custom `RetryStream`, `with_retry`, and exponential backoff (~200 LOC) even though `backon` is already in the workspace dependencies. `goose` uses standard retry patterns; Runie should too. The provider replay tests already cover the “retry only before the first streamed event” semantics, so replacing the implementation is low-risk.

## Acceptance Criteria

- [ ] Delete `crates/runie-provider/src/retry.rs` and its tests.
- [ ] Use `backon::Retryable` (or `reqwest-retry`) to retry provider requests before streaming starts.
- [ ] Preserve the current behavior: retry only until the first `ProviderEvent` is emitted; never retry mid-stream.
- [ ] Existing provider replay tests for MiniMax and other providers still pass.
- [ ] `cargo test --workspace` succeeds after the change.
- [ ] `cargo check --workspace` succeeds with no new warnings.

## Tests

### Layer 1 — State/Logic
- [ ] `retry_succeeds_after_transient_failure` — a mocked provider that fails once then streams succeeds after backoff.
- [ ] `no_retry_after_first_event` — once the first event is emitted, later failures are propagated, not retried.

### Layer 2 — Event Handling
- [ ] N/A.

### Layer 3 — Rendering
- [ ] N/A.

### Layer 4 — Provider Replay / Mock-Tool E2E
- [ ] `minimax_replay_retries_match` — replay the existing MiniMax SSE fixtures through the new retry wrapper and assert identical event sequences.

## Files touched

- `crates/runie-provider/src/retry.rs`
- `crates/runie-provider/src/lib.rs`
- `crates/runie-provider/src/openai/mod.rs` (or whichever module wires `with_retry`)
- `crates/runie-provider/Cargo.toml`

## Notes

- `ctx7` confirms `backon` supports async retry with custom backoff strategies and is widely used.
- The archived task `tasks/archive/replace-retry-with-backon.md` claimed this was done, but the custom code is still present. This task supersedes the archived one.
- Rejected: keep the custom module to avoid a dependency — `backon` is already declared and is the standard choice.
