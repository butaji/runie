# Replace custom retry modules with `backon`

**Status**: todo
**Milestone**: R4
**Category**: Core / State
**Priority**: P1

**Depends on**: none
**Blocks**: none

## Summary

Delete `crates/runie-core/src/retry.rs` and `crates/runie-provider/src/retry.rs` and replace the custom retry/backoff logic with the `backon` crate.

## Acceptance Criteria

- `backon` is added to workspace dependencies.
- Both custom retry modules are removed.
- All existing callers are migrated to `backon::Retryable` or an equivalent declarative retry API.
- Retry behavior is preserved: max attempts, exponential backoff, jitter, and transient/permanent error classification.
- `cargo check --workspace` is green with no new warnings.

## Tests

- **Layer 1**: Pure retry-policy unit tests (exponential backoff math, max attempts, jitter bounds).
- **Layer 2**: Event handling for retry-exhausted errors.
- **Layer 4**: Provider-replay test that simulates transient failures and recovery without real network calls.
