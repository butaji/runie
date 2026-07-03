# Standardize session persistence on JSONL

## Status

`done` — SQLite store does not exist. JSONL is the only persistence format.

## Description

Session persistence currently mixes JSONL with a custom `SqliteStore`. SQLite is deferred for now; JSONL must be the single canonical runtime store. Remove or fold `SqliteStore` into the JSONL session path so there is one persistence format.

### Implementation

- `crates/runie-core/src/session/sqlite_store.rs` does not exist
- Only `session/store.rs` with JSONL + fs2 advisory locks is used
- `tests/arch_guardrails.rs` explicitly excludes `session/sqlite_store.rs` as non-existent

## Acceptance criteria

- [x] **Unit tests** — JSONL round-trips session metadata, messages, and durable events. (`store.rs` tests)
- [x] **E2E tests** — `SessionLoaded`, `SessionSaved`, and `SessionDeleted` events still work after removing the SQLite path.
- [x] **Live run tests** — Save and resume a session in tmux; verify no SQLite files are created.

## Tests

### Unit tests
- Store round-trips metadata, messages, and durable events through JSONL.

### E2E tests
- `SessionLoaded`, `SessionSaved`, `SessionDeleted` events still work.

### Live run tests
- Save a session in tmux, list the sessions directory, and confirm only JSONL files are present.
