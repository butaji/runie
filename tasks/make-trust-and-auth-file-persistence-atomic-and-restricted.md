# Make trust and auth file persistence atomic and restricted

## Status

`done`

## Context

`crates/runie-core/src/trust.rs:37-44` wrote `~/.runie/trust.json` with plain `std::fs::write`, no lock, no temp+rename, and no permission restrictions. `crates/runie-core/src/auth/storage.rs:146-159` wrote the auth fallback with default world-readable permissions on Unix.

## Changes

- Created `crates/runie-core/src/io/atomic_write.rs` with `atomic_write()` helper that:
  - Creates temp file in same directory
  - Acquires exclusive fs2 advisory lock
  - Writes content and fsyncs
  - Sets Unix permissions to 0o600
  - Atomically renames to target
- Created `crates/runie-core/src/io/mod.rs` module
- Added `io` module to `lib.rs`
- Updated `trust.rs::save()` to use atomic_write helper
- Updated `auth/storage.rs::save_to_file()` to use atomic_write helper

## Acceptance Criteria

- [x] Add a small persistence helper used by both trust and auth modules.
- [x] Write to temp file, `fsync`, rename atomically, under `fs2` lock.
- [x] Set `0o600` on Unix.
- [x] Tests cover concurrent writers and permission checks.

## Tests

- **Layer 1 — State/Logic:** Unit tests for atomic writes, locks, and permissions (3 tests in `io::atomic_write`).
- **Layer 4 — E2E:** Trust and auth storage tests pass (11 trust tests, 9 auth tests).
