# Reconsider redb session store

**Status**: todo
**Milestone**: R4
**Category**: Sessions
**Priority**: P2

**Depends on**: none
**Blocks**: none

## Description

`redb` was adopted in the done task `adopt-redb-session-store` to replace JSONL + atomic rename for session persistence. Reversal argument under the YAGNI / stdlib posture:

- The durable event log is append-only. `SessionStore::append` and `append_batch` are the only write paths; there is no random-access update of existing events.
- `SessionIndex` (the metadata catalog) is already plain JSON in `sessions.json`, not redb.
- Each session gets its own `.redb` file, so redb's cross-table transactional guarantees buy little over a single append-only file.
- redb pulls in a C-free but substantial embedded DB stack (page cache, B+tree, MVCC) that stdlib file IO (`OpenOptions::append`, `BufWriter`, atomic rename for the index) covers for an append-only workload.

Either (a) revert to stdlib append-only JSONL + atomic-rename index, or (b) document a concrete workload that needs redb (e.g. future random-access event edits, transactional meta+events across many sessions) and keep it.

## Acceptance Criteria

- [ ] Decision made: EITHER
  - (a) **Revert** — `session_store.rs` rewritten over stdlib `fs` (append-only JSONL per session, atomic-rename `sessions.json` index); `redb` removed from `runie-core/Cargo.toml` and `[workspace.dependencies]`; migration reads existing `.redb` files one time and writes `.jsonl`; OR
  - (b) **Keep + document** — a concrete future workload justifying redb is written into `session_store.rs` module docs and this task's notes.
- [ ] If (a): `rg "redb::" crates/` returns zero hits; `Cargo.lock` no longer pulls `redb` transitive crates.
- [ ] If (a): existing sessions on disk are migrated without user action (read `.redb`, write `.jsonl`, rename `.redb` to `.redb.migrated`).
- [ ] `cargo check --workspace` succeeds with no new warnings.
- [ ] `cargo test --workspace` succeeds.

## Tests

### Layer 1 — State/Logic
- [ ] `append_event_writes_jsonl_line` — `SessionStore::append` produces one JSON line per event in `<id>.jsonl`.
- [ ] `load_events_round_trips` — events written then read back are byte-identical (role, content, timestamp, durable variant).
- [ ] `append_batch_atomic` — a batch of N events either fully appears or none appear (write to temp file, atomic rename).
- [ ] `index_load_handles_missing_file` — `SessionIndex::load` on a fresh data dir returns an empty index, not an error.

### Layer 2 — Event Handling
- [ ] `session_actor_persists_durable_events` — `SessionActor` still filters durable events and appends them to the new store (existing test stays green).

### Layer 3 — Rendering
- N/A.

### Layer 4 — Smoke / Crash
- [ ] `smoke_migrate_redb_to_jsonl` — given a fixture `.redb` session file, the new store reads it, writes `.jsonl`, and `load_events` returns the same sequence.
- [ ] `smoke_large_session_append` — appending 10 000 events to one session stays under 1s and does not grow memory unbounded.

## Files touched

- `crates/runie-core/src/session_store.rs` (rewrite if option a)
- `crates/runie-core/Cargo.toml` (remove `redb` if option a)
- `Cargo.toml` (remove `redb` from `[workspace.dependencies]` if option a)
- `crates/runie-core/src/session_index.rs` (unchanged — already JSON)
- `crates/runie-core/src/actors/session_store/actor.rs` (call sites unchanged, the `SessionStore` API stays the same)

## Notes

`adopt-redb-session-store` notes mention "atomic batch appends via single write transaction" and "automatic JSONL migration on first open". A revert must preserve both properties: batch atomicity via temp-file + rename, and a one-time read of any `.redb` file found at startup. If option (b) is chosen, link the justification back here and close this task as `wontfix` with a note.
