# Unify session store and replay index with `rusqlite`

**Status**: todo
**Milestone**: R4
**Category**: Sessions
**Priority**: P0

**Depends on**: none
**Blocks**: centralize-test-fixtures-and-mocks

## Description

Session persistence is currently split between ad-hoc JSON file dumps and a custom replay index. Move the canonical store to a single SQLite database using `rusqlite`: sessions, messages, checkpoints, and the replay index all live in one schema, and the custom persistence helpers are removed.

## Acceptance Criteria

- [ ] JSON file-based session store is replaced by `rusqlite`.
- [ ] Schema migrations are versioned and applied automatically on open.
- [ ] All session CRUD operations use prepared statements via the shared store.
- [ ] Replay index queries are expressed as SQL against the same database.
- [ ] `cargo test --workspace` succeeds after the change.
- [ ] `cargo check --workspace` succeeds with no new warnings.

## Tests

### Layer 1 — State/Logic
- [ ] `session_store_roundtrip` — create, update, and query a session purely in memory.
- [ ] `checkpoint_migration` — schema migration produces the expected replay index.

### Layer 2 — Event Handling
- [ ] `config_actor_loads_session_db` — ConfigActor events open the SQLite path and report readiness.

### Layer 3 — Rendering
- [ ] N/A — persistence has no TUI output.

### Layer 4 — Provider Replay / Mock-Tool E2E
- [ ] `replay_index_restores_turn` — a captured turn can be reloaded from SQLite and replayed through the agent.

## Files touched

- `crates/runie-core/src/session.rs`
- `crates/runie-core/src/store.rs`
- `crates/runie-agent/src/session_actor.rs`
- `crates/runie-testing/src/fixtures.rs`

## Notes

- Use `rusqlite` with the `bundled` feature to avoid system SQLite dependencies.
- Keep the database path configurable; default to `~/.local/share/runie/sessions.db` via `etcetera` or `dirs`.
- Out of scope: cloud sync or encryption.
