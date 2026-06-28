# Unify session store and index into a single persistence backend

**Status**: todo
**Milestone**: R1
**Category**: Sessions
**Priority**: P0

**Depends on**: delete-dead-actor-modules-and-custom-trait
**Blocks**: none

## Description

`crates/runie-core/src/session/store.rs` (450 LOC), `session/index.rs` (341 LOC), and `session/replay.rs` (354 LOC) implement a custom two-file persistence layer: one JSONL file per session for events and a separate `sessions.json` metadata index. The two files can drift, the index does linear scans, and atomic writes are hand-rolled with temp-file + rename. `thClaws` uses JSONL with `fs2` advisory locks and a headered single file. Runie should move session metadata and events into a single headered JSONL file with `fs2` advisory locks to remove the custom index and atomic-write code. SQLite is explicitly deferred for now.

## Acceptance Criteria

- [ ] Use a single headered JSONL file with `fs2` advisory locks as the persistence backend.
- [ ] Replace the two-file store+index design with one source of truth.
- [ ] Preserve session metadata, event sequence, replay, search, and `/save`/`/load` behavior.
- [ ] Provide a one-time migration from existing `~/.runie/sessions/*.jsonl` + `sessions.json` to the new backend.
- [ ] Delete the custom temp-file/rename atomic-write code; the single file + `fs2` locks replace it.
- [ ] `cargo test --workspace` succeeds after the change.
- [ ] `cargo check --workspace` succeeds with no new warnings.

## Tests

### Layer 1 — State/Logic
- [ ] `save_and_load_session_round_trip` — a session with events is saved and loaded identically.
- [ ] `session_search_finds_by_name` — session list search works after unification.
- [ ] `migration_reads_legacy_jsonl_and_index` — existing session files migrate correctly.
- [ ] `fs2_lock_prevents_corruption` — concurrent writers are serialized by advisory locks.

### Layer 2 — Event Handling
- [ ] N/A.

### Layer 3 — Rendering
- [ ] N/A.

### Layer 4 — Provider Replay / Mock-Tool E2E
- [ ] `session_replay_after_persistence_refactor` — replay an existing session fixture through the new backend and assert the same `AppState` events.

## Files touched

- `crates/runie-core/src/session/store.rs`
- `crates/runie-core/src/session/index.rs`
- `crates/runie-core/src/session/replay.rs`
- `crates/runie-core/src/event/durable.rs`
- `crates/runie-core/src/session/tree.rs`
- `crates/runie-core/Cargo.toml` (add `fs2`, remove `tempfile` if no longer needed)

## Notes

- SQLite is explicitly deferred; use `fs2` advisory locks and a single headered JSONL file.
- The header stores session metadata; the body is the JSONL event stream.
- `fs2` provides cross-process locking without a separate database runtime.
- Coordinate with `delete-dead-actor-modules-and-custom-trait.md` to avoid merge conflicts in actor spawn sites.
