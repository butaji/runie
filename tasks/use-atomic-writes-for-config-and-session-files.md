# Use atomic writes for config and session files

## Status

`done` — `tempfile::NamedTempFile` atomic writes are implemented (`io/atomic_write.rs`).

## Description

Config and session writes are not atomic; a crash can leave empty or partial files. Use `atomicwrites` or `tempfile::NamedTempFile::persist`.

### Implementation

`crates/runie-core/src/io/atomic_write.rs` implements atomic writes:
- Uses `tempfile::NamedTempFile` with `persist()`
- Sets restrictive permissions (mode 0o600)
- Tests verify atomic replacement, overwriting, and permissions
- Used by `auth/storage.rs` for config and auth files

## Tests

### Unit tests
- Atomic replacement under failure injection.

### E2E tests
- Save/reload round-trip.

### Live tmux tests
- Save session, kill process, resume.
