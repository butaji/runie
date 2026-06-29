# Store provider API keys in OS keyring, not plain `config.toml`

**Status**: todo
**Milestone**: R6
**Category**: Security / Configuration
**Priority": P1

**Depends on**: replace-xor-auth-with-keyring
**Blocks**: type-and-unify-provider-model-layer

## Description

Provider API keys currently live as plaintext strings in `~/.runie/config.toml` (`[model_providers.*].api_key`). Move them to the OS keyring (service `runie`, account `provider:<name>`) and support an env-var fallback for CI/headless. Optionally wrap secrets in `secrecy::SecretString` to prevent leakage in logs/traces.

## Acceptance Criteria

- [ ] Add `keyring` (and optionally `secrecy`) to workspace deps.
- [ ] On config load, resolve `[model_providers.*].api_key` from keyring if the value is a keyring alias or empty.
- [ ] Fall back to env var `{PROVIDER}_API_KEY`.
- [ ] Provide a migration that moves existing plaintext keys into the keyring and rewrites config.
- [ ] Replace `api_key: String` with `secrecy::SecretString` or a small newtype.
- [ ] `cargo test --workspace` succeeds after the change.
- [ ] `cargo check --workspace` succeeds with no new warnings.

## Tests

### Layer 1 — State/Logic
- [ ] `keyring_resolution_falls_back_to_env` — missing keyring entry falls back to env.
- [ ] `secret_string_does_not_leak_in_debug` — `Debug` output is redacted.

### Layer 2 — Event Handling
- [ ] `config_actor_loads_provider_key_from_keyring` — `ConfigActor` resolves a key on load.

### Layer 4 — Provider Replay / Mock-Tool E2E
- [ ] `provider_replay_uses_keyring_key` — a provider replay turn succeeds with a keyring-stored key.

## Files touched

- `crates/runie-core/src/config/mod.rs`
- `crates/runie-core/src/config/provider_config.rs`
- `crates/runie-core/src/provider/config.rs`
- `crates/runie-core/src/auth.rs` (or new `crates/runie-core/src/secrets.rs`)
- `crates/runie-provider/src/config/mod.rs`

## Notes

- Peer codebases (Goose, thClaws) use `keyring` with file/env fallback.
- thClaws bundles all provider keys in one keychain entry to minimize macOS prompts; consider that approach if the user configures many providers.
- This should land before heavy provider/model typing so the key type can be `SecretString`.
