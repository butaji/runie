# Delete DynProvider and ProviderConfigBox wrappers

## Status

`done`

## Context

`DynProvider` wraps `BuiltProvider` solely for backward compatibility; `ProviderConfigBox` is a cloneable wrapper around `Arc<dyn ProviderConfig>`.

## Changes

- `DynProvider` was a type alias (`pub type DynProvider = BuiltProvider;`) in `runie-provider/src/lib.rs`. Removed it entirely — callers use `BuiltProvider` directly.
- `ProviderConfigBox` was a struct with `Arc<dyn ProviderConfig>` as inner. Replaced with a type alias: `pub type ProviderConfigBox = Arc<dyn ProviderConfig>;`. This preserves binary compatibility for any external re-exports while eliminating the wrapper struct.

## Acceptance Criteria

- [x] Remove `DynProvider` and `ProviderConfigBox` definitions.
- [x] Update all call sites and tests.
- [x] `cargo check --workspace` passes.

## Files changed

- `crates/runie-core/src/proto/provider.rs` — replaced `ProviderConfigBox` struct with type alias
- `crates/runie-provider/src/lib.rs` — removed `DynProvider` alias; updated function signatures to use `Arc<dyn ProviderConfig>`
- `crates/runie-provider/src/factory.rs` — updated `DynProviderFactory::build` and `resolve_credentials` to use `Arc<dyn ProviderConfig>`
- `crates/runie-provider/src/config/mod.rs` — updated `ProviderConfigResolver` to use `Arc<dyn ProviderConfig>`
- `crates/runie-provider/src/config/tests.rs` — updated test to use `Arc::new(cfg) as Arc<dyn ProviderConfig>`
- `crates/runie-testing/src/replay_provider.rs` — updated `dyn_replay_provider` to return `BuiltProvider`
- `crates/runie-testing/src/fixtures.rs` — updated `mock_provider` to return `BuiltProvider`

## Tests

- **Layer 4 — E2E:** All provider and agent tests pass (`cargo test --workspace` with `--test-threads=1`).
