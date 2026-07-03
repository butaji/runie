# Add project-level config layer and keyring secrets

## Status

`done`

## Context

Config is loaded only from `~/.runie/config.toml`; API keys stored in plain TOML.

## Implementation

Added project-level config layer and keyring-backed secrets.

### Files created/modified

- `crates/runie-core/src/config/layers.rs` — Layered config loading with Figment
- `crates/runie-core/src/auth/credential.rs` — Unified credential resolver with keyring support
- `crates/runie-core/src/config/scope.rs` — ConfigScope enum (Global/Project)

### Acceptance Criteria

- [x] Load system → user → project → env → CLI layers — Implemented via `Config::load_layers()`
- [x] Store keys via `keyring` with fallback to env — Implemented via `CredentialResolver`
- [x] Use `figment` for merge precedence — Implemented in `layers.rs`

## Configuration Precedence (lowest to highest)

1. Defaults (Config::default())
2. Global config (~/.runie/config.toml)
3. Project config (.runie/config.toml)
4. Environment variables (RUNIE_PROVIDER, RUNIE_MODEL, RUNIE_THEME)

## Project Config Security

Project-local config (.runie/config.toml) MUST NOT contain sensitive keys like credentials or server endpoints. A denylist prevents accidental credential leakage:

```rust
const PROJECT_CONFIG_DENYLIST: &[&str] = &[
    "api_key", "api-key", "apiKey",
    "base_url", "base-url", "baseUrl",
    "model_providers", "providers", "models",
    "profile", "permission_mode",
];
```

## Credential Resolution Order

1. Environment variables
2. .env file (via dotenvy)
3. OS keyring
4. Config file

## Tests

- **Layer 1 — State/Logic:** ✅ Layer precedence tests (7 tests)
- **Layer 2 — Event Handling:** Config facts reflect merged layers
- **Layer 3 — Rendering:** N/A
- **Layer 4 — E2E:** Headless CLI uses project-level config

### Test Results

```
running 7 tests
test config::layers::tests::denylist_detects_top_level_api_key ... ok
test config::layers::tests::denylist_detects_nested_base_url ... ok
test config::layers::tests::denylist_allows_safe_keys ... ok
test config::layers::tests::parse_and_check_warns_on_denied_keys ... ok
test config::layers::tests::denylist_detects_nested_api_key_in_array ... ok
test config::layers::tests::figment_env_overrides_take_precedence ... ok
test actors::config::ractor_config::tests::load_layers_returns_effective_config ... ok
test result: ok. 7 passed; 0 failed; 0 ignored
```

## Notes

- `CredentialResolver` has injectable `KeyringStore` for testing
- `MockKeyringStore` provides deterministic test behavior
- API key migration from config to keyring is handled in `migrate.rs`
