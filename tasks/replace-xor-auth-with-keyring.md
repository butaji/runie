# Replace XOR-obfuscated auth storage with OS keyring

**Status**: todo
**Milestone**: R1
**Category**: Architecture / Security
**Priority**: P0

**Depends on**: none
**Blocks**: none

## Description

`crates/runie-core/src/auth.rs` implements a custom XOR cipher keyed off `hostname` + `$HOME` to “obfuscate” stored tokens. This is security theater (~120 LOC). `goose` uses the `keyring` crate for cross-platform credential storage; `thClaws` defaults to the OS keychain with `.env` fallback. Runie should store secrets in the OS keyring and fall back to a plain file or env var only in headless/CI contexts.

## Acceptance Criteria

- [ ] Replace `AuthStorage` encryption/decryption with `keyring` lookups keyed by service + account (e.g. `"runie"` / `provider_id`).
- [ ] Add a compile-time or runtime fallback for headless/CI environments where no keyring is available (env var or plain `.runie/auth.json`).
- [ ] Remove the `hostname`/`$HOME`-derived machine key and the XOR/base64 code.
- [ ] Migrate existing `~/.runie/auth.json` files on first read, or document a manual migration step.
- [ ] `cargo test --workspace` succeeds after the change.
- [ ] `cargo check --workspace` succeeds with no new warnings.

## Tests

### Layer 1 — State/Logic
- [ ] `store_and_load_token` — round-trip a token through the new storage.
- [ ] `fallback_when_keyring_unavailable` — when `keyring` returns an error, the fallback path works.
- [ ] `migration_reads_legacy_auth_json` — an existing XOR-encoded file is decrypted and re-stored with `keyring`.

### Layer 2 — Event Handling
- [ ] N/A.

### Layer 3 — Rendering
- [ ] N/A.

### Layer 4 — Provider Replay / Mock-Tool E2E
- [ ] N/A.

## Files touched

- `crates/runie-core/src/auth.rs`
- `crates/runie-core/src/auth/tests.rs` (if any)
- `crates/runie-core/Cargo.toml`
- `crates/runie-cli/src/login.rs` (or whichever command triggers storage)
- `crates/runie-core/src/config/mod.rs` (secret field usage)

## Notes

- `ctx7` confirms `keyring` supports Linux (Secret Service), macOS (Keychain), Windows (Credential Manager), FreeBSD, OpenBSD, and iOS.
- Use `keyring` with the `vendored` feature if static builds are needed (as `goose` does).
- This task is independent of the actor/config migration; it can land in parallel.
- Consider wrapping secret strings in `secrecy::SecretString` (or `zeroize::Zeroizing`) so `Debug` logs and panic traces do not leak keys.
