# Move provider registry and model catalog into runie-provider

**Status**: done
**Milestone**: R4
**Category**: Architecture / Actors
**Priority**: P0

**Depends on**: unify-provider-modules, move-chatmessage-to-shared-crate
**Blocks**: none

## Summary

The circular dependency issue has been resolved by introducing a `ProviderConfig` trait in `runie-protocol`. This allows `runie-provider` to depend on `runie-core` for `Config` while `runie-core` depends on `runie-provider` for registry/catalog without creating a cycle.

## What was done

### ProviderConfig trait in runie-protocol

Added `ProviderConfig` trait and `ProviderConfigBox` wrapper to `runie-protocol/src/provider.rs`:

```rust
pub trait ProviderConfig: Send + Sync + fmt::Debug {
    fn resolve_api_key(&self, provider: &str) -> Option<String>;
    fn resolve_base_url(&self, provider: &str) -> Option<String>;
}

pub struct ProviderConfigBox {
    inner: std::sync::Arc<dyn ProviderConfig>,
}
```

### Config implements ProviderConfig

Added `runie-core/src/config/provider_config.rs` with the implementation:

```rust
impl runie_protocol::ProviderConfig for Config {
    fn resolve_api_key(&self, provider: &str) -> Option<String> { ... }
    fn resolve_base_url(&self, provider: &str) -> Option<String> { ... }
}
```

### Registry and catalog moved to runie-provider

- `crates/runie-provider/src/registry/` - Provider registry (ProviderMeta, ModelMeta, known_providers)
- `crates/runie-provider/src/catalog/` - Model catalog (ModelCapabilities, ModelInfo)
- `crates/runie-provider/resources/models/` - YAML files for each provider

### Backward compatibility

- `runie-core` re-exports registry and catalog types from `runie-provider`
- `runie-provider/src/config/mod.rs` updated to use `ProviderConfigBox`

## Acceptance Criteria

- [x] `ProviderConfig` trait added to `runie-protocol`
- [x] `ProviderConfigBox` wrapper added for cloneable type erasure
- [x] `Config` implements `ProviderConfig` in `runie-core`
- [x] Provider registry moved to `runie-provider/src/registry/`
- [x] Model catalog moved to `runie-provider/src/catalog/`
- [x] `runie-core` re-exports registry and catalog from `runie-provider`
- [x] No circular dependency
- [x] `cargo test --workspace` succeeds (2701 tests pass)
- [x] `cargo check --workspace` succeeds with no new warnings

## Files touched

- `crates/runie-protocol/src/provider.rs` (new)
- `crates/runie-protocol/src/message/` (new - ChatMessage already moved)
- `crates/runie-core/src/config/provider_config.rs` (new)
- `crates/runie-provider/src/registry/` (new)
- `crates/runie-provider/src/catalog/` (new)
- `crates/runie-provider/resources/models/*.yaml` (new)
- `crates/runie-provider/src/config/mod.rs` (updated)
- `crates/runie-provider/src/factory.rs` (updated)
- `crates/runie-provider/src/lib.rs` (updated)
