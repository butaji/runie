# Decouple AppState from view-AST cache

**Status**: done
**Milestone**: R4
**Category**: Architecture / Actors
**Priority**: P0

**Depends on**: fold-state-into-model-state
**Blocks**: rename-core-ui-to-view

## Summary

Removed `cached_view` and `cached_view_gen` from `AppState`. The view projection (Element/Post/Feed) is now built on-demand in `build_view_cache()` when `ensure_fresh()` or `snapshot()` is called. This decouples `AppState` from the view cache, moving the caching responsibility to `UiActor`.

## Changes

### `crates/runie-core/src/model/state/app_state.rs`
- Removed `cached_view: Option<ViewCache>` and `cached_view_gen: u64` fields
- Removed `__with_cache_for_test()` helper (no longer needed)
- `AppState` now holds only domain state; view projection is built on-demand

### `crates/runie-core/src/model/cache/mod.rs`
- `view_cache()` renamed to `build_view_cache()` which returns a temporary `ViewCache`
- Removed `CacheData` struct (no longer needed without caching)
- Simplified `snapshot_mouse_impl()` to take raw data instead of `ViewCache`
- Removed test that relied on cached elements (now built on-demand)

### `crates/runie-core/src/view/mod.rs`
- Updated module documentation to reflect that AppState no longer caches the feed

### `crates/runie-core/src/tests/snapshot_optimization.rs`
- Updated Arc pointer stability tests to test correctness rather than internal caching

### `crates/runie-tui/src/tests/render/transient.rs`
- Updated to use `__with_transient_test()` instead of removed `__with_cache_for_test()`

### `crates/runie-core/tests/arch_guardrails.rs`
- New file with architecture guardrail tests

## Acceptance Criteria

- [x] `AppState` no longer has a field of type `LazyCache` / `Feed` / `Element` / `Post`
- [x] The view projection is built on-demand, not cached in AppState
- [x] `crates/runie-core` exports Element/Feed/Post from view module, but AppState doesn't own them
- [x] `arch_guardrails.rs` gains a test asserting AppState struct fields contain no view cache
- [x] `cargo test --workspace` succeeds
- [x] `cargo check --workspace` succeeds with no new warnings

## Notes

The view projection is still computed in `AppState::build_view_cache()` when building snapshots. This is a partial implementation of option (b) - the projection stays in core but AppState no longer caches it. A future refactor could move the projection entirely to UiActor (option (a)).
