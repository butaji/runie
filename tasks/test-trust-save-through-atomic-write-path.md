# Test trust save through atomic write path

## Status

`done`

## Context

`crates/runie-core/src/trust.rs:113-138` writes the trust file directly with `std::fs::write` instead of `TrustManager::save()`, bypassing the new atomic-write path.

## Goal

Use `tm.save()` in the test and assert `0o600` permissions on Unix.

## Implementation

Updated `save_load_roundtrip` test to:
1. Set `RUNIE_TEST_CONFIG_DIR` env var to the temp directory so `save()` uses the correct path
2. Use `tm.save()` instead of direct JSON serialization
3. Verify the file exists after save

Added new `save_sets_restricted_permissions` test that:
1. Saves via `tm.save()` 
2. Verifies Unix permissions are 0o600 (user read/write only)

## Acceptance Criteria

- [x] Replace direct write with `tm.save()`.
- [x] Assert file permissions.
- [x] Test passes.

## Files Touched

- `crates/runie-core/src/trust.rs` - Updated tests

## Tests

- **Layer 1 — State/Logic:** `save_load_roundtrip` and `save_sets_restricted_permissions` pass.

- **Layer 2 — Event Handling:** N/A.

- **Layer 3 — Rendering:** N/A.

- **Layer 4 — E2E:** Trust tests pass as part of `cargo test --workspace`.

- **Live tmux testing session (required):** N/A.

## Verification

```
cargo test --package runie-core trust -- --nocapture
# All 15 trust-related tests pass

cargo test --workspace
# All workspace tests pass

cargo check --workspace
# No warnings or errors
```
