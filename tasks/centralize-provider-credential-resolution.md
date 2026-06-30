# Centralize provider credential resolution

**Status**: done
**Milestone**: R7
**Category**: Configuration
**Priority**: P2

**Depends on**: unify-provider-credential-resolution-with-dotenvy, store-provider-api-keys-in-keyring-not-config
**Blocks**: type-and-unify-provider-model-layer

## Description

Provider API key/base URL resolution is implemented in at least four places with slight differences. Create a single `ProviderCredentialResolver` in `runie-provider` or `runie-core` that applies the env → dotenvy → keyring → config priority consistently.

## Acceptance Criteria

- [x] Single resolver implements env/dotenvy/keyring/config priority.
- [x] All provider configs use the resolver.
- [x] Remove manual `.env` re-parsing in `runie-provider/src/config/mod.rs`.
- [x] `cargo check --workspace` and `cargo test --workspace` pass.

## Tests

### Layer 1 — State/Logic
- [x] `resolver_priority_env_over_keyring` — env wins over keyring.
- [x] `resolver_fallback_to_config` — config used when env/keyring absent.

### Layer 4 — Provider Replay / Mock-Tool E2E
- [x] `provider_replay_uses_resolver` — replay uses centralized resolution.

## Files touched

- `crates/runie-core/src/config/provider_config.rs`
- `crates/runie-core/src/provider/config.rs`
- `crates/runie-provider/src/config/mod.rs`
- `crates/runie-provider/src/factory.rs`
- `crates/runie-core/src/auth.rs`

## Notes

- Use `dotenvy::dotenv()` once and then `std::env::vars()`.
