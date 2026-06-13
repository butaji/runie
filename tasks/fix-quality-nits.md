# Fix Quality and Performance Nits

**Status**: todo
**Milestone**: R3
**Category**: Core Architecture
**Priority**: P3

## Description

A collection of small, non-blocking quality issues spotted during the
architecture and code review:

1. **`SpeedWindow::evict_old` uses `Vec::remove(0)`** in
   `crates/runie-core/src/state.rs`, making speed-window updates O(n²)
   over the number of recorded events. A `VecDeque` or circular buffer
   should be used.
2. **`AppState::snapshot()` is rebuilt every tick`** even when the state
   has not changed. The render channel already drops stale snapshots, so
   emitting identical snapshots wastes CPU.
3. **`update/dialog.rs::insert_at_ref` has convoluted string slicing**
   for `@`-file insertion. It can be simplified by operating on
   character indices consistently.
4. **Stale doc example in `commands/dsl/builder.rs`** references
   `build_login_root(state)`, noted in `tasks/index.json` under
   `sync-docs`.

## Acceptance Criteria

- [ ] `SpeedWindow` uses an O(1) amortized eviction strategy.
- [ ] `AppState::snapshot()` returns a cached snapshot when
  `!is_dirty()` (or a generation counter proves no change).
- [ ] `insert_at_ref` is refactored for clarity and has unit tests.
- [ ] The stale `build_login_root(state)` doc example is fixed.
- [ ] `cargo build --workspace` and `cargo test --workspace` succeed.

## Tests

### Layer 1 — State/Logic
- [ ] `speed_window_eviction_is_linear_amortized` — many records do not
  degrade performance.
- [ ] `snapshot_reuses_cache_when_clean` — calling `snapshot()` twice
  without changes yields the same data.
- [ ] `insert_at_ref_cases` — covers prefix, suffix, middle, and empty
  prefix insertion.

### Layer 2 — Event Handling
- [ ] No event behavior changes.

### Layer 3 — Rendering
- [ ] No rendering changes.

### Layer 4 — Smoke
- [ ] `./dev.sh` still runs.

## Notes

**Out of scope:**
- Large refactors (handled in other tasks).
- Adding new features.

## Verification

```bash
cargo test --workspace
cargo clippy --workspace
```
