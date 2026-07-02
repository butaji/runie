# Add tracing to `runie-provider`

**Status**: done
**Milestone**: R7
**Category**: Observability
**Priority**: P2

**Depends on**: initialize-tracing-subscriber-in-binaries
**Blocks**: none

## Description

`runie-provider` had no tracing instrumentation. This task adds spans around key operations:

### Retries (`retry.rs`)
- Added `#[instrument]` pattern via `tracing::Instrument` trait
- Spans around `with_retry_config` with attempt count and delay metadata
- Debug logging for retry start/success/failure

### API Validation (`lib.rs`)
- Spans around `validate_api_key_with_timeout` with base_url metadata
- Debug logging for validation success/failure
- Trace-level logging for HTTP requests and responses

### SSE Streaming (`openai/stream.rs`)
- Info spans around `openai_event_stream` with model and message count
- Debug spans around `build_eventsource_with_retry`
- Debug spans around `stream_sse_events` with event counts
- Trace-level logging for SSE frame parsing

### Request Building (`openai/request.rs`)
- Debug spans around `build_request_body` with model and message count
- Trace-level logging for message normalization

## Acceptance Criteria

- [x] Add spans around retries (`with_retry_config`)
- [x] Add spans around API validation (`validate_api_key_with_timeout`, `fetch_models`)
- [x] Add spans around SSE parsing (`openai_stream`, `build_eventsource`, `stream_sse_events`)
- [x] Add spans around request building (`build_request_body`)
- [x] `cargo test --workspace` succeeds
- [x] `cargo check --workspace` succeeds with no new warnings

## Tests

### Layer 1 — State/Logic
- [x] Existing tests pass (no behavior change, only instrumentation added)

### Layer 4 — Provider Replay / Mock-Tool E2E
- [x] All 132 provider tests pass (4 replay tests, 128 unit/integration tests)
- [x] Tracing has zero overhead on test execution

## Files Touched

- `crates/runie-provider/Cargo.toml` — added `tracing.workspace = true`
- `crates/runie-provider/src/lib.rs` — added `use tracing::Instrument`; spans in `validate_api_key_with_timeout`, `fetch_models`
- `crates/runie-provider/src/retry.rs` — added `use tracing::Instrument`; spans in `with_retry_config`
- `crates/runie-provider/src/openai/stream.rs` — added `use tracing::Instrument`; spans in `openai_event_stream`, `build_eventsource_with_retry`, `stream_sse_events`, `parse_sse_line`
- `crates/runie-provider/src/openai/request.rs` — spans in `build_request_body`

## Notes

- Spans use standard naming convention: `info_span!` for user-facing operations, `debug_span!` for internal operations
- Fields include relevant metadata (model, base_url, message_count, etc.)
- All existing tests continue to pass, confirming no performance regression
- The `tracing` crate is already a workspace dependency, so no Cargo.lock changes needed
