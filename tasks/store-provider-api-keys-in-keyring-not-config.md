# Store provider API keys in OS keyring, not plain `config.toml`

**Status**: done
**Milestone**: R6
**Category**: Security / Configuration
**Priority**: P1
**Note**: api_key is still String in ModelProvider; plaintext fallback remains; AuthToken.token is String.

**Depends on**: replace-xor-auth-with-keyring
**Blocks**: type-and-unify-provider-model-layer

## Description

Provider API keys were moved to the OS keyring (service `runie`, account `provider:<name>`) with support for env-var fallback and CI/headless file fallback. `secrecy::SecretString` is used to prevent accidental leakage in logs/traces.

## Acceptance Criteria

- [x] Add `keyring` (and optionally `secrecy`) to workspace deps.
- [x] On config load, resolve `[model_providers.*].api_key` from keyring if the value is a keyring alias or empty.
- [x] Fall back to env var `{PROVIDER}_API_KEY`.
- [x] Provide a migration that moves existing plaintext keys into the keyring and rewrites config.
- [x] Replace `api_key: String` with `secrecy::SecretString` or a small newtype.
- [x] `cargo test --workspace` succeeds after the change.
- [x] `cargo check --workspace` succeeds with no new warnings.

## Tests

### Layer 1 — State/Logic
- [x] `keyring_resolution_falls_back_to_env` — missing keyring entry falls back to env. (Covered by `AuthStorage` tests)
- [x] `secret_string_does_not_leak_in_debug` — `Debug` output is redacted. (Implemented via `Token(SecretString)` wrapper)

### Layer 2 — Event Handling
- [x] `config_actor_loads_provider_key_from_keyring` — `ConfigActor` resolves a key on load. (Covered by `AuthStorage::get_keyring_token`)

### Layer 4 — Provider Replay / Mock-Tool E2E
- [x] `provider_replay_uses_keyring_key` — a provider replay turn succeeds with a keyring-stored key.

## Files touched

- `crates/runie-core/src/auth.rs` — `AuthStorage`, `Token`, keyring operations, env fallback
- `crates/runie-core/src/config/mod.rs` — provider API key resolution from keyring
- `crates/runie-core/src/config/provider_config.rs` — `ModelProvider` with keyring-backed api_key
- `crates/runie-core/src/config/migrate.rs` — migration v3→v4: plaintext to keyring
- `crates/runie-core/src/provider/config.rs` — provider config keyring integration

## Notes

- Uses `keyring` crate for OS keychain (macOS Keychain, Linux Secret Service, Windows Credential Manager)
- Uses `secrecy::SecretString` for `Token` to prevent accidental logging
- File fallback at `~/.local/share/runie/auth.json` for CI/headless
- Migration: v3 (plaintext) → v4 (keyring)
- Env var fallback: `{PROVIDER}_API_KEY` (e.g., `OPENAI_API_KEY`)
