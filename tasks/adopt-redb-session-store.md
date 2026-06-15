# Adopt `redb` for Session Persistence

**Status**: todo
**Milestone**: R3
**Category**: Sessions
**Priority**: P2

**Depends on**: event-bus-jsonl-persistence
**Blocks**: (none)

## Description

Replace the JSONL event log + JSON metadata index in `crates/runie-core/src/session_store.rs` with `redb`, a pure-Rust ACID embedded key-value store. This gives atomic batches, indexing, and removes hand-rolled atomic writes and JSON merging.

## Acceptance Criteria

- [ ] `redb` is added as a dependency.
- [ ] `session_store.rs` stores durable events and metadata in `redb`.
- [ ] Sessions can be loaded and replayed in order.
- [ ] Migration path from existing JSONL sessions is provided or documented.
- [ ] Schema versioning is handled.
- [ ] `cargo test --workspace` succeeds.

## Tests

### Layer 1 — State/Logic
- [ ] `redb_appends_and_replays_events` — write durable events, reload, assert order.
- [ ] `redb_atomic_batch_survives_crash` — partial batch does not corrupt the store.
- [ ] `redb_migrates_jsonl_session` — old JSONL session imports correctly.

### Layer 2 — Event Handling
- [ ] `session_actor_persists_to_redb` — durable events end up in `redb`.

### Layer 3 — Rendering
N/A.

### Layer 4 — Smoke / Crash
- [ ] `smoke_session_resume_from_redb` — run binary, send message, kill, resume, verify conversation restored.

## Files touched

- `crates/runie-core/Cargo.toml`
- `crates/runie-core/src/session_store.rs`
- `crates/runie-core/src/session_actor.rs`

## Notes

- Wrap sync `redb` calls in `spawn_blocking` to avoid blocking the async runtime.
- Consider `redb` vs `rusqlite` if relational queries become useful later.
- See `docs/CRATE_DECISIONS.md`.
