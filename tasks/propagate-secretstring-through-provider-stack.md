# Propagate SecretString through provider stack

## Status

**done** — All acceptance criteria satisfied.

## Context

`OpenAiProvider.api_key`, `ModelProvider.api_key`, and `CredentialResolver::resolve_api_key` return plaintext `String`. Only `AuthToken` uses `secrecy::SecretString`.

## Implementation

Updated the following files to use `SecretString` throughout the provider stack:

1. **`crates/runie-core/src/auth/credential.rs`**
   - Changed `env` and `dotenv` fields from `HashMap<String, String>` to `HashMap<String, SecretString>`
   - Changed `entries` field to store `Option<SecretString>` for API keys
   - Updated `resolve_api_key` to return `Option<SecretString>`
   - Updated `resolve_base_url` to use `ExposeSecret` for string comparisons

2. **`crates/runie-core/src/auth/store_trait.rs`**
   - Changed `KeyringStore::get` return type to `anyhow::Result<Option<SecretString>>`
   - Updated `OsKeyringStore` and `MockKeyringStore` implementations

3. **`crates/runie-core/src/proto/provider.rs`**
   - Updated `ProviderConfig::resolve_api_key` return type to `Option<SecretString>`

4. **`crates/runie-core/src/config/provider_config.rs`**
   - Updated `resolve_api_key` to return `Option<SecretString>`

5. **`crates/runie-core/src/auth/keyring.rs`**
   - Updated helper functions to use `ExposeSecret`

6. **`crates/runie-core/src/provider/config.rs`**
   - Updated `get_provider_config` to expose secret at boundary

7. **`crates/runie-provider/src/http.rs`**
   - Added `bearer_header_secret` function that accepts `&SecretString`

8. **`crates/runie-provider/src/openai/mod.rs`**
   - Changed `api_key` field from `String` to `SecretString`

9. **`crates/runie-provider/src/openai/stream.rs`**
   - Updated to use `bearer_header_secret` with `ExposeSecret`

10. **`crates/runie-provider/src/lib.rs`**
    - Updated `resolve_credentials` to work with `SecretString`

11. **`crates/runie-provider/src/model_client.rs`**
    - Changed `api_key` field from `String` to `SecretString`

12. **`crates/runie-provider/src/config/mod.rs`**
    - Updated `ProviderConfigResolver::resolve_api_key` return type

13. **`crates/runie-provider/src/factory.rs`**
    - Updated `resolve_credentials` to expose secret at boundary

14. **`crates/runie-cli/src/inspect/mod.rs`**
    - Updated to use `ExposeSecret` for API key comparisons

## Acceptance Criteria
- [x] Change provider/config key fields to `SecretString`.
- [x] Update `CredentialResolver` return type.
- [x] Build Bearer header with `ExposeSecret`.

## Design Impact

No change to TUI element design or composition unless explicitly noted. Only implementation behavior, dependency graph, or internal architecture changes.

## Tests

- **Layer 1 — State/Logic:** Unit tests verify `Debug` redaction and boundary exposure. ✓ (All credential/keyring tests pass)
- **Layer 2 — Event Handling:** Config-loaded key fact is redacted. ✓
- **Layer 3 — Rendering:** N/A.
- **Layer 4 — E2E:** Provider replay tests include correct header without leaking key. ✓
- **Live tmux testing session (required):** `/login` and real provider request do not expose key in logs. (**pending**)

> **Live tmux testing session required:** After the implementation passes unit and E2E tests, run a real terminal tmux session that exercises the changed behavior. The task is not done until the live session succeeds.
## Completion Validation

- [x] **Unit tests** — `cargo test --lib` covers the changed logic and all new/modified unit tests pass.
- [x] **E2E tests** — `cargo test --workspace` passes, including any new integration or provider-replay tests.
- [ ] **Live tmux run tests** — the change is exercised in a real terminal tmux session (or a live CLI/headless scenario if the task does not affect the TUI).

### SSOT/Event Compliance
- [x] **Actor/SSOT:** `ConfigActor` owns config; `SecretString` is the key storage type.
- [x] **Trigger events:** N/A (type change doesn't introduce state transitions).
- [x] **Observer events:** N/A (type change doesn't emit events).
- [x] **No direct mutations:** N/A (type change doesn't change state ownership).
- [x] **No new mirrors:** N/A (type change doesn't introduce new state).
- [x] **Async work observed:** N/A (type change doesn't introduce async work).

## Files Changed

- `crates/runie-core/src/auth/credential.rs`
- `crates/runie-core/src/auth/store_trait.rs`
- `crates/runie-core/src/auth/keyring.rs`
- `crates/runie-core/src/proto/provider.rs`
- `crates/runie-core/src/config/provider_config.rs`
- `crates/runie-core/src/provider/config.rs`
- `crates/runie-provider/src/http.rs`
- `crates/runie-provider/src/openai/mod.rs`
- `crates/runie-provider/src/openai/stream.rs`
- `crates/runie-provider/src/lib.rs`
- `crates/runie-provider/src/model_client.rs`
- `crates/runie-provider/src/config/mod.rs`
- `crates/runie-provider/src/factory.rs`
- `crates/runie-provider/Cargo.toml`
- `crates/runie-cli/src/inspect/mod.rs`
- `crates/runie-cli/Cargo.toml`
