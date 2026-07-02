# Centralize provider HTTP timeouts and retry constants

## Status

`done` (2026-07-02)

## Description

`reqwest` request/connect timeouts (`120`, `10`) were duplicated in three locations:
- `runie-provider/src/openai/mod.rs`
- `runie-provider/src/model_client.rs`
- `runie-core/src/actors/provider/factory.rs`

Status-code classification (`401`, `403`, `429`, `>=500`) was already centralized in `runie-core/src/provider/provider_trait.rs::classify_http_status`, and `retry.rs` calls it. `RetryConfig` was already honored by `with_retry_config`.

## Changes

1. Added `REQUEST_TIMEOUT` and `CONNECT_TIMEOUT` constants to `runie-core/src/provider/provider_trait.rs`.
2. Re-exported from `runie-core/src/provider/mod.rs`.
3. Updated `factory.rs` to import and use the centralized constants.
4. Updated `runie-provider/src/lib.rs` to re-export the constants from `runie-core`.
5. Updated `model_client.rs` and `openai/mod.rs` to use the centralized constants.

## Acceptance criteria

- [x] All provider HTTP clients use the same named constants.
- [x] Status classifier is centralized in `ProviderError::classify_http_status`.
- [x] `RetryConfig` is honored by `with_retry_config`.
- [x] All tests pass.

## Tests

- [x] `cargo test --workspace` passes.
- [x] No new warnings from `cargo check --workspace`.
