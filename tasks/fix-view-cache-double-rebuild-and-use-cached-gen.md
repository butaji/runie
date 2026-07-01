# Fix view cache double rebuild and use cached_gen

## Status

`done`

## Context

`model/cache/mod.rs:220-236` builds a `ViewCache`, then `snapshot_feed()` builds another from scratch. `cached_gen` is written but never read.

## Goal

Keep one `ViewCache` in `ViewState`, compare `view.message_gen` with `cache.cached_gen`, and reuse. Delete the second build.

## Acceptance Criteria

- [x] `ensure_fresh()` updates the cache and sets `cached_gen`.
- [x] `snapshot_feed()` reuses the cache when `message_gen == cached_gen`.
- [x] Long feeds render without O(n) rebuild per frame.
- [x] Tests verify cache reuse.

## Design Impact

No change to TUI element design or composition. Only render performance changes.

## Tests

- **Layer 1 — State/Logic:** Unit tests for cache hit/miss and `cached_gen` invalidation.
- **Layer 2 — Event Handling:** `Snapshot` events produce the same view.
- **Layer 3 — Rendering:** `TestBackend` snapshots unchanged.
- **Layer 4 — E2E:** Provider replay fixture with many messages passes.
- **Live tmux validation:** Scroll a long conversation; rendering stays responsive.

## Implementation

1. Added `pub(crate) cached_feed: Option<ViewCache>` to `ViewState` (`model/state/view.rs`).
2. `ensure_fresh()` stores the built `ViewCache` in `self.view_mut().cached_feed = Some(cache.clone())`.
3. `snapshot_feed()` checks `cached_feed.cached_gen == message_gen` — reuses if match, rebuilds if stale.
4. Added `Debug` derive to `ViewCache` (needed by `ViewState`'s `Debug` derive).
5. Two new unit tests: `test_cached_feed_reuse_on_gen_match` and `test_cached_feed_none_initially`.

## Completion Validation

- [x] **Unit tests** — `cargo test --lib` covers the changed logic and all new/modified unit tests pass.
- [x] **E2E tests** — `cargo test --workspace` passes, including any new integration or provider-replay tests.
- [x] **Live tmux run tests** — the change is exercised in a real terminal tmux session (or a live CLI/headless scenario if the task does not affect the TUI).

## Files Changed

- `crates/runie-core/src/model/state/view.rs` — added `cached_feed` field
- `crates/runie-core/src/model/view_cache.rs` — added `Debug` derive
- `crates/runie-core/src/model/cache/mod.rs` — store and reuse cache
- `crates/runie-core/src/tests/snapshot_optimization.rs` — two new tests
