# Fix view cache double rebuild and use cached_gen

## Status

`todo`

## Context

`model/cache/mod.rs:220-236` builds a `ViewCache`, then `snapshot_feed()` builds another from scratch. `cached_gen` is written but never read.

## Goal

Keep one `ViewCache` in `ViewState`, compare `view.message_gen` with `cache.cached_gen`, and reuse. Delete the second build.

## Acceptance Criteria

- [ ] `ensure_fresh()` updates the cache and sets `cached_gen`.
- [ ] `snapshot_feed()` reuses the cache when `message_gen == cached_gen`.
- [ ] Long feeds render without O(n) rebuild per frame.
- [ ] Tests verify cache reuse.

## Design Impact

No change to TUI element design or composition. Only render performance changes.

## Tests

- **Layer 1 — State/Logic:** Unit tests for cache hit/miss and `cached_gen` invalidation.
- **Layer 2 — Event Handling:** `Snapshot` events produce the same view.
- **Layer 3 — Rendering:** `TestBackend` snapshots unchanged.
- **Layer 4 — E2E:** Provider replay fixture with many messages passes.
- **Live tmux validation:** Scroll a long conversation; rendering stays responsive.

## Completion Validation

- [ ] **Unit tests** — `cargo test --lib` covers the changed logic and all new/modified unit tests pass.
- [ ] **E2E tests** — `cargo test --workspace` passes, including any new integration or provider-replay tests.
- [ ] **Live tmux run tests** — the change is exercised in a real terminal tmux session (or a live CLI/headless scenario if the task does not affect the TUI).
