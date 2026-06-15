# Fix SessionStore Atomic Writes

**Status**: todo
**Milestone**: R3
**Category**: Sessions
**Priority**: P1

**Depends on**: (none)
**Blocks**: event-bus-jsonl-persistence

## Description

`crates/runie-core/src/session_store.rs` documents atomic writes ("write to temp file, then rename") but `SessionStore::append` opens the target `.jsonl` file directly and calls `sync_all()` after every line. It also creates an unused `temp_path`. This is both non-atomic and slow.

## Acceptance Criteria

- [ ] `SessionStore::append` writes to a temp file and renames it to the target path.
- [ ] The unused `temp_path` variable is removed or used correctly.
- [ ] `sync_all()` is not called on every append (or is justified and tested).
- [ ] `cargo test --workspace` succeeds.

## Tests

### Layer 1 — State/Logic
- [ ] `session_store_atomic_write_survives_crash` — simulated crash mid-write leaves the original file intact.
- [ ] `session_store_appends_and_replays_events` — existing append/replay test still passes.

### Layer 2 — Event Handling
N/A — persistence only.

### Layer 3 — Rendering
N/A — persistence only.

### Layer 4 — Smoke / Crash
N/A — covered by Layer 1.

## Files touched

- `crates/runie-core/src/session_store.rs`

## Notes

- A simple atomic append: write the new line to `<id>.jsonl.tmp`, then `fs::rename` to `<id>.jsonl`.
- Consider buffering/batching if `sync_all()` was intended for durability; document the trade-off.
