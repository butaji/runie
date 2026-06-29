# Replace XOR-obfuscated auth storage with OS keyring

**Status**: done
**Milestone**: R1
**Category**: Architecture / Security
**Priority**: P0

**Depends on**: none
**Blocks**: none

## Description

`crates/runie-core/src/auth.rs` implements a custom XOR cipher keyed off `hostname` + `$HOME` to "obfuscate" stored tokens. This is security theater (~120 LOC). `goose` uses the `keyring` crate for cross-platform credential storage; `thClaws` defaults to the OS keychain with `.env` fallback. Runie should store secrets in the OS keyring and fall back to a plain file or env var only in headless/CI contexts.

## Acceptance Criteria

- [x] Replace `AuthStorage` encryption/decryption with `keyring` lookups keyed by service + account (e.g. `"runie"` / `provider_id`).
- [x] Add a compile-time or runtime fallback for headless/CI environments where no keyring is available (env var or plain `.runie/auth.json`).
- [x] Remove the `hostname`/`$HOME`-derived machine key and the XOR/base64 code.
- [x] Migrate existing `~/.runie/auth.json` files on first read, or document a manual migration step.
- [x] `cargo test --workspace` succeeds after the change.
- [x] `cargo check --workspace` succeeds with no new warnings.

## Tests

### Layer 1 — State/Logic
- [x] `store_and_load_token` — round-trip a token through the new storage.
- [x] `fallback_when_keyring_unavailable` — when `keyring` returns an error, the fallback path works.
- [x] `migration_reads_legacy_auth_json` — an existing XOR-encoded file is decrypted and re-stored with `keyring`.

### Layer 2 — Event Handling
- [x] N/A.

### Layer 3 — Rendering
- [x] N/A.

### Layer 4 — Provider Replay / Mock-Tool E2E
- [x] N/A.

## Files touched

- `crates/runie-core/src/auth.rs` — rewritten to use keyring with file fallback
- `crates/runie-core/Cargo.toml` — added keyring and secrecy dependencies
- `Cargo.toml` (workspace) — added keyring and secrecy dependencies
- `crates/runie-tui/src/app_init.rs` — updated to use new `providers()` method

## Notes

- `ctx7` confirms `keyring` supports Linux (Secret Service), macOS (Keychain), Windows (Credential Manager), FreeBSD, OpenBSD, and iOS.
- Used `keyring` with the `vendored` feature for static builds.
- Added `secrecy::SecretString` wrapper to prevent accidental token leakage in logs.
- Migration from legacy XOR-encoded `auth.json` is handled by `migrate_legacy_auth()` which stores tokens in keyring and backs up the old file.
