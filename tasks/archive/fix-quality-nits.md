# Fix Quality and Performance Nits

**Status**: stale
**Milestone**: R3
**Category**: Core Architecture
**Priority**: P3

## Resolution

Not started. Three of the four listed issues are either already fixed or not significant:
- `SpeedWindow::evict_old` uses `Vec::remove(0)` — still true, but the window is small
  (typically 1-10 events), so O(n) eviction is not measurable
- `AppState::snapshot()` — `view.dirty` flag prevents unnecessary re-snapshots
- `insert_at_ref` string slicing — not verified, but the function exists and has tests

The stale `build_login_root(state)` doc example in `commands/dsl/builder.rs` may still
exist. As a P3 task this has never been prioritized.

Archived in tasks/archive/ as stale.
