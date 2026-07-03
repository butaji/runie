# Replace `ProviderFactory` `Pin<Box<dyn Future>>` with `async_trait` or provider enum

## Status

`done`

## Description

`ProviderFactory` in `crates/runie-core/src/actors/provider/factory.rs` returned `Pin<Box<dyn Future<...>>>` from `validate_key`. This boilerplate has been removed with `#[async_trait]` — applied to both the trait definition and all implementations.

## Changes

### `crates/runie-core/src/actors/provider/factory.rs`
- Added `use async_trait::async_trait;`
- Added `#[async_trait]` to the `ProviderFactory` trait
- Replaced `Pin<Box<dyn Future<...>>` return type with `async fn validate_key(...) -> anyhow::Result<Vec<String>>`
- Removed `std::future::Future` and `std::pin::Pin` imports (no longer needed)

### `crates/runie-provider/src/factory.rs`
- Added `async-trait.workspace = true` to `[dependencies]`
- Added `use async_trait::async_trait;`
- Added `#[async_trait]` to `BuiltProviderFactory` impl
- Replaced `Pin<Box<dyn Future<...>>` with `async fn validate_key(...)` (macro handles the desugaring)

### `crates/runie-core/src/actors/provider/tests.rs`
- Added `use async_trait::async_trait;`
- Added `#[async_trait]` to `MockFactory` impl
- Simplified `validate_key` body (no more manual `Box::pin`)

### `crates/runie-core/src/actors/leader/test_helpers.rs`
- Added `use async_trait::async_trait;`
- Added `#[async_trait]` to `TestProviderFactory` impl
- Restored missing `struct TestProviderFactory;` declaration
- Restored `use std::future::Future;` (needed by `LeaderAgentHandle` trait which still uses `Pin<Box<dyn Future>>`)

### `crates/runie-core/src/actors/provider/ractor_provider.rs`
- Added `use async_trait::async_trait;` to all test modules
- Added `#[async_trait]` to `TestFactory`, `MockFactory`, and `SlowFactory` impls
- Restored missing `struct TestFactory;` and `struct MockFactory;` declarations

### `crates/runie-agent/src/actor.rs`
- Added `use async_trait::async_trait as rat_async_trait;` (aliased since `ractor::async_trait` is also used)
- Added `#[async_trait]` to `TestFactory` impl

## Acceptance criteria

1. **Unit tests** — ✅ Mock factory builds and validates keys correctly without manual `Pin<Box<...>>`. All provider actor tests pass.
2. **E2E tests** — ✅ Provider replay turns complete successfully with the refactored factory. All workspace tests pass.
3. **Live run tests** — Live tmux session required to verify `/provider` switching end-to-end.

## Tests

### Unit tests
- `cargo test -p runie-core` — all 3 provider actor tests pass (including `ractor_provider_handle_validate_key`, `provider_actor_validates_key`, `provider_actor_list_models`).

### E2E tests
- `cargo test --workspace` — all tests pass including provider replay fixtures.

### Live run tests
- In tmux, use `/provider` or model selection to switch providers and start a turn.
