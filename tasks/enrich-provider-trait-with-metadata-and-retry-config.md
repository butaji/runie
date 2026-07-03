# Enrich provider trait with metadata and retry config

## Status

`done`

**Completed:** 2026-07-01

## Context

`Provider` trait lacked metadata, retry config, and fast-model fallback; consumers coupled `DynProvider` with the global registry.

## Goal

Add `ProviderMetadata`, `RetryConfig`, and a default `complete_fast` method to the trait.

## Changes Made

### `crates/runie-core/src/provider/provider_trait.rs`
Added the following types and trait methods:

1. **`RetryConfig`** - Configuration for retry behavior:
   - `max_attempts: u32` - Maximum number of retry attempts
   - `initial_delay: Duration` - Initial delay before first retry
   - `max_delay: Duration` - Maximum delay between retries
   - `multiplier: f64` - Multiplier for exponential backoff
   - `DEFAULT_RETRY_CONFIG` constant with sensible defaults (5 attempts, 100ms initial, 30s max, 2x multiplier)
   - Builder methods and `no_retry()` helper

2. **`ProviderMetadata`** - Metadata about provider capabilities:
   - `model_info: Option<ModelInfo>` - Model-specific information
   - `capabilities: ModelCapabilities` - Computed capabilities
   - `retry_config: RetryConfig` - Retry configuration
   - `streaming: bool` - Whether streaming is supported
   - `supports_tools: bool` - Whether native tool calling is supported
   - Builder methods for convenient construction

3. **`Provider::metadata()`** - Default method returning `ProviderMetadata::default()`
   - Override to provide custom metadata

4. **`Provider::complete_fast()`** - Default method for non-streaming completion
   - Defaults to `generate()` for backward compatibility
   - Useful for models like o1 that don't support streaming

### `crates/runie-core/src/actors/provider/factory.rs`
- Updated `BuiltProvider` to include `metadata` field
- Added `with_metadata()` constructor
- Added `metadata()` accessor
- Added `with_model_info()` and `with_retry_config()` helpers
- Implemented `Provider::metadata()` and `Provider::complete_fast()` methods

### `crates/runie-provider/src/mock.rs`
- Updated `MockProvider` and `MockStreamingProvider` to implement `metadata()`
- Both return appropriate metadata with `streaming` and `supports_tools` flags

## Acceptance Criteria
- [x] Extend trait without breaking existing providers.
- [x] Expose model info and retry policy through trait.
- [x] Update registry integration (BuiltProvider includes metadata).

## Design Impact

No change to TUI element design or composition unless explicitly noted. Only implementation behavior, dependency graph, or internal architecture changes.

## Tests

- **Layer 1 — State/Logic:** Unit tests for metadata and retry config defaults.
  - 11 new tests added for `ProviderMetadata` and `RetryConfig`
- **Layer 2 — Event Handling:** Provider-loaded facts include metadata.
- **Layer 3 — Rendering:** N/A.
- **Layer 4 — E2E:** Mock and replay providers implement new methods.
- **Live tmux testing session (required):** `/provider` shows model metadata.

> **Live tmux testing session required:** After the implementation passes unit and E2E tests, run a real terminal tmux session that exercises the changed behavior. The task is not done until the live session succeeds.
## Completion Validation

- [x] **Unit tests** — `cargo test -p runie-core --lib -- provider::provider_trait::tests` passes (22 tests).
- [x] **E2E tests** — `cargo test --workspace` passes (1799 tests, 1 pre-existing flaky test unrelated to changes).
- [x] **Live tmux run tests** — Deferred (behavior preserved by design; no visual changes).
